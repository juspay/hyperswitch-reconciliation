use api_models::linking_processor::LinkingTriggerResponse;
use common_enums::EntryLinkSide;
use common_types::entry_linking::{
    EntryLinkingData, EntryLinkingDataV1, LinkingFailureData, LinkingFailureDataV1,
};
use common_types::metadata_schema::ProcessingModeProvider;
use common_types::recon_rule::entry_field::EntryField;
use common_utils::{encryption::Encryption, logging};
use diesel_models::search_entry_link_key::{SearchEntryLinkKey, SearchEntryLinkKeyNew};
use diesel_models::{
    entry::{Entry as DieselEntry, EntryUpdate},
    entry_link::EntryLinkNew,
};
use error_stack::{Report, ResultExt};
use masking::Secret;
use recon_domain_models::{
    behaviour::ReverseConversion, entry::DomainEntry, transaction::DomainTransaction,
};
use thiserror::Error;

use crate::db::search_entry_link_key::SearchEntryLinkSearchCriteria;
use crate::{
    db::{
        metadata_schema::MetadataSchemaInterface, transaction::TransactionInterface,
        transformation::TransformationConfigInterface,
    },
    errors::linking_processor::{LinkingProcessorError, LinkingProcessorResult},
    process_linking_or_fail,
    routes::AppState,
};

pub mod implementations;
pub mod link_orchestration;

#[derive(Debug, Clone, Error)]
pub enum LinkingProcessingError {
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Missing transformation config id")]
    TransformationConfigMissing,

    #[error("Database error occurred")]
    DatabaseError,

    #[error("No Link Config found for processing")]
    NoLinkConfigFound,

    #[error("Missing search identifier value for field: {0}")]
    MissingSearchIdentifierValue(EntryField),

    #[error("Multiple Posted links found")]
    MultiplePostedLinksFound,
}

type LinkProcessingResult<T> = Result<T, Report<LinkingProcessingError>>;

pub async fn process_entry_linking_core(
    state: &AppState,
    entry: DieselEntry,
) -> LinkingProcessorResult<LinkingTriggerResponse> {
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(entry.merchant_id.clone());

    let domain_entry: DomainEntry = process_linking_or_fail!(
        async {
            entry
                .clone()
                .convert_back(&merchant_encryptor)
                .await
                .change_context(LinkingProcessingError::InternalError(
                    "Failed to decrypt entry".to_string(),
                ))
        },
        state,
        &entry
    );

    let transaction = state
        .store
        .find_transaction_by_transaction_id_and_version(
            &domain_entry.transaction_id,
            domain_entry.version,
        )
        .await
        .change_context(LinkingProcessorError::InternalServerError)?;

    let domain_transaction: DomainTransaction = transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(LinkingProcessorError::InternalServerError)?;

    let transformation_config = match domain_entry.transformation_id.clone() {
        Some(config_id) => state
            .store
            .find_transformation_config_by_id_and_profile_id(
                &config_id,
                &domain_transaction.profile_id,
            )
            .await
            .change_context(LinkingProcessorError::InternalServerError)?,
        None => {
            let reason = process_linking_error_context(
                state,
                Report::new(LinkingProcessingError::TransformationConfigMissing),
                &entry,
            )
            .await?;
            return Ok(LinkingTriggerResponse::Failure(reason));
        }
    };

    let metadata_schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(
            &transformation_config.metadata_schema_id,
            &domain_transaction.profile_id,
        )
        .await
        .change_context(LinkingProcessorError::InternalServerError)?;

    let link_processing_context = process_linking_or_fail!(
        async {
            link_orchestration::link_orchestration(
                state,
                &domain_entry,
                domain_transaction,
                metadata_schema
                    .schema_data
                    .get_processing_mode()
                    .get_entry_linking_side(),
            )
            .await
        },
        state,
        &entry
    );

    let entry_link_context = process_linking_or_fail!(
        async {
            link_processing_context
                .link_config
                .create_context(state, domain_entry, link_processing_context.entry_link_side)
                .await
        },
        state,
        &entry
    );

    process_linking_or_fail!(
        async {
            link_processing_context
                .link_config
                .process_context(state, entry_link_context)
                .await
        },
        state,
        &entry
    );

    Ok(LinkingTriggerResponse::Success)
}

pub async fn process_linking_error_context(
    state: &AppState,
    error: Report<LinkingProcessingError>,
    entry: &DieselEntry,
) -> LinkingProcessorResult<String> {
    logging::error!("Linking failed for entry {}: {:?}", &entry.id, error);

    let linking_data = EntryLinkingData::V1(EntryLinkingDataV1::Failed(LinkingFailureData::V1(
        LinkingFailureDataV1 {
            error_type: error.current_context().clone().into(),
        },
    )));

    let conn = state
        .store
        .get_connection()
        .await
        .change_context(LinkingProcessorError::InternalServerError)?;

    DieselEntry::update_by_id(
        &conn,
        &entry.id,
        EntryUpdate::LinkingUpdate { data: linking_data }.into(),
    )
    .await
    .change_context(LinkingProcessorError::InternalServerError)?;

    Ok(error.to_string())
}

pub enum EntryLinkContext {
    UnLinked {
        entry_link_key_new: SearchEntryLinkKeyNew,
    },
    Linked {
        entry_link_key: SearchEntryLinkKeyNew,
        new_entry_links: Vec<EntryLinkNew>,
        matching_link_key_ids: Vec<String>,
    },
    PostedLinked {
        entry_link_key: SearchEntryLinkKeyNew,
        new_entry_link: EntryLinkNew,
        matched_link_key_id: String,
    },
}

pub trait LinkingProcessor {
    async fn create_context(
        &self,
        state: &AppState,
        domain_entry: DomainEntry,
        entry_link_side: EntryLinkSide,
    ) -> LinkProcessingResult<EntryLinkContext>;

    fn create_posted_context(
        &self,
        domain_entry: &DomainEntry,
        entry_link_side: EntryLinkSide,
        matching_entry_link_keys: Vec<SearchEntryLinkKey>,
        encrypted_value: Encryption,
        hashed_value: Secret<String>,
    ) -> LinkProcessingResult<EntryLinkContext>;

    fn create_non_posted_context(
        &self,
        domain_entry: &DomainEntry,
        entry_link_side: EntryLinkSide,
        matching_entry_link_keys: Vec<SearchEntryLinkKey>,
        encrypted_value: Encryption,
        hashed_value: Secret<String>,
    ) -> LinkProcessingResult<EntryLinkContext>;

    fn get_search_criteria(
        &self,
        domain_entry: &DomainEntry,
        entry_link_side: &EntryLinkSide,
        hashed_value: Secret<String>,
    ) -> SearchEntryLinkSearchCriteria;

    async fn process_context(
        &self,
        state: &AppState,
        context: EntryLinkContext,
    ) -> LinkProcessingResult<SearchEntryLinkKey>;
}
