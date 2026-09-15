use api_models::recon_engine::ReconTriggerResponse;
use async_bb8_diesel::AsyncConnection;
use common_enums::{BalanceDirection, Currency};
use common_types::{
    entry::FieldMismatchDetail,
    recon_rule::entry_field::EntryField,
    staging_entry::{ManualReviewData, ManualReviewDataV1, StagingEntryData, StagingEntryDataV2},
    units::MinorUnit,
};
use common_utils::logging;
use diesel_models::{
    errors::DatabaseError,
    staging_entry::{
        StagingEntry as DieselStagingEntry, StagingEntryNew, StagingEntryUpdate,
        StagingEntryVersionUpdate,
    },
};
use error_stack::{Report, ResultExt};
use recon_domain_models::{
    behaviour::ReverseConversion, staging_entry::DomainStagingEntry, transaction::DomainTransaction,
};
use thiserror::Error;

use crate::{
    db::{metadata_schema::MetadataSchemaInterface, transformation::TransformationConfigInterface},
    domain::recon_engine::{
        flows::ReconStrategyExt,
        process_duplicate::check_for_duplicate,
        rule_orchestration::{ProcessingDecision, ProcessingDecisionOutput},
    },
    errors::{conversions::StorageErrorExt, ReconEngineError, ReconEngineResult},
    process_or_fail,
    routes::AppState,
    utils::transaction::TransactionExt,
};

pub mod flows;
pub mod implementations;
pub mod process_duplicate;
pub mod rule_orchestration;
pub mod utils;

#[derive(Debug, Clone, Error)]
pub enum ProcessingError {
    #[error("No rules found for processing")]
    NoRulesFound,

    #[error("Missing search identifier value: {0}")]
    MissingSearchIdentifierValue(EntryField),

    #[error("Missing match field: {0}")]
    MissingMatchField(EntryField),

    #[error("Database error occurred")]
    DatabaseError,

    #[error("Staging entry currency mismatch: staging entry has {staging_currency:?} but account has {account_currency:?}")]
    StagingEntryCurrencyMismatch {
        staging_currency: Currency,
        account_currency: Currency,
    },

    #[error("No expectation entry found")]
    NoExpectationEntryFound,

    #[error("Multiple expected entries found with transaction ids: {transaction_ids:?}")]
    MultipleExpectedEntriesFound { transaction_ids: Vec<String> },

    #[error("Duplicate entry detected")]
    DuplicateEntry,

    #[error("Missing unique field for duplicate detection: {0}")]
    MissingUniqueField(EntryField),

    #[error("Missing grouping field: {0}")]
    MissingGroupingField(EntryField),

    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Debug, Clone)]
pub enum MatchRuleValidation<T> {
    Passed,
    Failed(T),
}

#[derive(Debug, Clone)]
pub enum AmountMismatch {
    Failed(AmountMismatchDetail),
    WithinTolerance(ToleranceDetail),
}

#[derive(Debug, Clone)]
pub struct AmountMismatchDetail {
    pub expected: MinorUnit,
    pub actual: MinorUnit,
    pub currency: Currency,
}

#[derive(Debug, Clone)]
pub struct ToleranceDetail {
    pub tolerance_amount: MinorUnit,
    pub currency: Currency,
    pub tolerance_balance_direction: BalanceDirection,
    pub tolerance_account_id: String,
}

#[derive(Debug, Clone)]
pub struct BalanceDirectionMismatch {
    pub expected: BalanceDirection,
    pub actual: BalanceDirection,
}

#[derive(Debug, Clone)]
pub struct CurrencyMismatch {
    pub expected: Currency,
    pub actual: Currency,
}

#[derive(Debug, Clone)]
pub struct MetadataMismatch {
    pub mismatched_fields: Vec<FieldMismatchDetail>,
}

pub type ProcessingResult<T> = Result<T, Report<ProcessingError>>;

pub async fn process_staging_entry_core_v2(
    state: &AppState,
    staging_entry: DieselStagingEntry,
    check_duplicate_exists: bool,
) -> ReconEngineResult<ReconTriggerResponse> {
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(staging_entry.merchant_id.clone());

    let domain_staging_entry: DomainStagingEntry = process_or_fail!(
        async {
            staging_entry
                .clone()
                .convert_back(&merchant_encryptor)
                .await
                .change_context(ProcessingError::InternalError(
                    "Failed to decrypt staging_entry".to_string(),
                ))
        },
        state,
        &staging_entry
    );

    let transformation_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &domain_staging_entry.transformation_id,
            &domain_staging_entry.profile_id,
        )
        .await
        .to_not_found_response(|| ReconEngineError::InternalServerError)?;

    let metadata_schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(
            &transformation_config.metadata_schema_id,
            &transformation_config.profile_id,
        )
        .await
        .to_not_found_response(|| ReconEngineError::InternalServerError)?;

    if check_duplicate_exists {
        process_or_fail!(
            check_for_duplicate(state, &domain_staging_entry, &metadata_schema),
            state,
            &staging_entry
        );
    }

    let processing_decision = process_or_fail!(
        rule_orchestration::rule_orchestration_v2(state, &domain_staging_entry, metadata_schema),
        state,
        &staging_entry
    );

    let ProcessingDecisionOutput {
        transaction,
        aging_config,
    } = match processing_decision {
        ProcessingDecision::ProcessAsTransaction {
            context,
            recon_rule,
        } => {
            let strategy = recon_rule.strategy;
            let prep_data = process_or_fail!(
                strategy.get_recon_engine_type().prep_transaction(
                    state,
                    domain_staging_entry.clone(),
                    *context
                ),
                state,
                &staging_entry
            );

            let transaction_context = process_or_fail!(
                prep_data.create_transaction_context(state),
                state,
                &staging_entry
            );

            ProcessingDecisionOutput {
                transaction: process_or_fail!(
                    transaction_context.process_transaction_context(state),
                    state,
                    &staging_entry
                ),
                aging_config: recon_rule.aging_config,
            }
        }
        ProcessingDecision::ProcessAsConfirmation {
            context,
            recon_rule,
        } => {
            let strategy = recon_rule.strategy;
            let prep_confirmation = process_or_fail!(
                strategy.get_recon_engine_type().prep_confirmation(
                    state,
                    domain_staging_entry.clone(),
                    *context
                ),
                state,
                &staging_entry
            );

            let confirmation_context = process_or_fail!(
                prep_confirmation.create_confirmation_context(state),
                state,
                &staging_entry
            );

            ProcessingDecisionOutput {
                transaction: process_or_fail!(
                    confirmation_context.process_confirmation_context(state),
                    state,
                    &staging_entry
                ),
                aging_config: recon_rule.aging_config,
            }
        }
    };

    let domain_transaction: DomainTransaction = transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(ReconEngineError::InternalServerError)?;

    let modified_by = domain_transaction
        .get_modified_by(state, staging_entry.merchant_id, staging_entry.profile_id)
        .await
        .change_context(ReconEngineError::InternalServerError)?;

    let recon_transaction = domain_transaction
        .to_response(&aging_config, modified_by)
        .change_context(ReconEngineError::InternalServerError)?;

    Ok(ReconTriggerResponse::Success(Box::new(recon_transaction)))
}

pub async fn process_error_context(
    state: &AppState,
    error: Report<ProcessingError>,
    staging_entry: &DieselStagingEntry,
) -> ReconEngineResult<String> {
    logging::error!(
        "Processing failed for staging entry {}: {:?}",
        &staging_entry.id,
        error
    );

    let manual_review_data = StagingEntryData::V2(StagingEntryDataV2::NeedsManualReview(
        ManualReviewData::V1(ManualReviewDataV1 {
            error_type: error.current_context().clone().into(),
        }),
    ));

    let conn = state
        .store
        .get_connection()
        .await
        .change_context(ReconEngineError::InternalServerError)?;

    conn.transaction_async(|conn| async move {
        // Step 1: Archive the existing staging entry
        let archive_update = StagingEntryUpdate::DiscardUpdate {
            original_data: staging_entry.data.clone(),
        };

        DieselStagingEntry::update_by_id(&conn, &staging_entry.id, archive_update.into())
            .await
            .map_err(|e| {
                logging::error!("Failed to archive staging entry: {:?}", e);
                *e.current_context()
            })?;

        // Step 2: Create new staging entry with manual review status
        let manual_review_staging_entry_new = StagingEntryNew::new_versioned(
            staging_entry.clone(),
            StagingEntryVersionUpdate::NeedsManualReview {
                data: manual_review_data,
            },
            None,
        );

        // Insert the manual review staging entry
        manual_review_staging_entry_new
            .insert(&conn)
            .await
            .map_err(|e| {
                logging::error!("Failed to insert manual review staging entry: {:?}", e);
                *e.current_context()
            })?;

        Result::<_, DatabaseError>::Ok(())
    })
    .await
    .change_context(ReconEngineError::InternalServerError)?;

    Ok(error.to_string())
}
