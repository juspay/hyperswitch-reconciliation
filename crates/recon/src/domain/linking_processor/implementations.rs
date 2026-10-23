use async_bb8_diesel::AsyncConnection;
use common_enums::{EntryLinkKeyStatus, EntryLinkSide, EntryLinkStatus};
use common_types::entry_linking::{EntryLinkingData, EntryLinkingDataV1};
use common_utils::{encryption::Encryption, logging};
use diesel_models::{
    entry::{Entry, EntryUpdate},
    entry_link::EntryLinkNew,
    errors::DatabaseError,
    link_config::LinkConfig,
    search_entry_link_key::{SearchEntryLinkKey, SearchEntryLinkKeyNew, SearchEntryLinkKeyUpdate},
};
use error_stack::ResultExt;
use masking::Secret;
use recon_domain_models::entry::DomainEntry;

use crate::{
    db::search_entry_link_key::{SearchEntryLinkKeyInterface, SearchEntryLinkSearchCriteria},
    domain::linking_processor::{
        EntryLinkContext, LinkProcessingResult, LinkingProcessingError, LinkingProcessor,
    },
    routes::AppState,
    utils::recon_engine::hash_string,
};

use self::link_matching_key::LinkSearchIdentifierExt;

pub mod link_matching_key;
pub mod link_trigger;

impl LinkingProcessor for LinkConfig {
    async fn create_context(
        &self,
        state: &AppState,
        domain_entry: DomainEntry,
        entry_link_side: EntryLinkSide,
    ) -> LinkProcessingResult<EntryLinkContext> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(domain_entry.merchant_id.clone());

        let value = match entry_link_side {
            EntryLinkSide::Transaction => self
                .link_search_identifier
                .extract_transaction_value(&domain_entry)?,
            EntryLinkSide::Confirmation => self
                .link_search_identifier
                .extract_confirmation_value(&domain_entry)?,
        };
        let hashed_value = hash_string(&value);

        let encrypted_value = encryptor.encrypt_string(value).await.change_context(
            LinkingProcessingError::InternalError("Failed to encrypt entry link value".to_string()),
        )?;

        let search_criteria =
            self.get_search_criteria(&domain_entry, &entry_link_side, hashed_value.clone());
        let matching_link_keys = state
            .store
            .list_search_entry_link_keys_for_linking(&search_criteria)
            .await
            .change_context(LinkingProcessingError::DatabaseError)?;

        if domain_entry.status.is_posted() {
            self.create_posted_context(
                &domain_entry,
                entry_link_side,
                matching_link_keys,
                encrypted_value.into(),
                hashed_value,
            )
        } else {
            self.create_non_posted_context(
                &domain_entry,
                entry_link_side,
                matching_link_keys,
                encrypted_value.into(),
                hashed_value,
            )
        }
    }

    fn create_posted_context(
        &self,
        domain_entry: &DomainEntry,
        entry_link_side: EntryLinkSide,
        matching_entry_link_keys: Vec<SearchEntryLinkKey>,
        encrypted_value: Encryption,
        hashed_value: Secret<String>,
    ) -> LinkProcessingResult<EntryLinkContext> {
        let unlinked_posted_keys = matching_entry_link_keys
            .into_iter()
            .filter(|key| key.status == EntryLinkKeyStatus::PostedOpen)
            .collect::<Vec<_>>();

        let status = match unlinked_posted_keys.as_slice() {
            [] => EntryLinkKeyStatus::PostedOpen,
            [_] => EntryLinkKeyStatus::PostedMatched,
            _ => return Err(LinkingProcessingError::MultiplePostedLinksFound.into()),
        };
        let entry_link_key_new = create_entry_link_key(
            self,
            domain_entry,
            entry_link_side,
            status,
            encrypted_value,
            hashed_value,
        );

        match unlinked_posted_keys.first() {
            None => Ok(EntryLinkContext::UnLinked { entry_link_key_new }),
            Some(matched_entry_link_key) => {
                let new_entry_link = create_entry_link(
                    &entry_link_key_new,
                    matched_entry_link_key,
                    entry_link_side,
                    EntryLinkStatus::Final,
                );

                Ok(EntryLinkContext::PostedLinked {
                    entry_link_key: entry_link_key_new,
                    new_entry_link,
                    matched_link_key_id: matched_entry_link_key.id.clone(),
                })
            }
        }
    }

    fn create_non_posted_context(
        &self,
        domain_entry: &DomainEntry,
        entry_link_side: EntryLinkSide,
        matching_link_keys: Vec<SearchEntryLinkKey>,
        encrypted_value: Encryption,
        hashed_value: Secret<String>,
    ) -> LinkProcessingResult<EntryLinkContext> {
        let status = if matching_link_keys.is_empty() {
            EntryLinkKeyStatus::Open
        } else {
            EntryLinkKeyStatus::Matched
        };

        let entry_link_key_new = create_entry_link_key(
            self,
            domain_entry,
            entry_link_side,
            status,
            encrypted_value,
            hashed_value,
        );

        if matching_link_keys.is_empty() {
            return Ok(EntryLinkContext::UnLinked { entry_link_key_new });
        }

        let new_entry_links = matching_link_keys
            .iter()
            .map(|matching_entry_link_key| {
                create_entry_link(
                    &entry_link_key_new,
                    matching_entry_link_key,
                    entry_link_side,
                    EntryLinkStatus::Provisional,
                )
            })
            .collect();

        Ok(EntryLinkContext::Linked {
            entry_link_key: entry_link_key_new,
            new_entry_links,
            matching_link_key_ids: matching_link_keys
                .iter()
                .map(|key| key.id.clone())
                .collect(),
        })
    }

    fn get_search_criteria(
        &self,
        domain_entry: &DomainEntry,
        entry_link_side: &EntryLinkSide,
        hashed_value: Secret<String>,
    ) -> SearchEntryLinkSearchCriteria {
        let target_key = match entry_link_side {
            EntryLinkSide::Transaction => self.link_search_identifier.get_transaction_field(),
            EntryLinkSide::Confirmation => self.link_search_identifier.get_confirmation_field(),
        };

        let statuses = EntryLinkKeyStatus::from_posted_status(domain_entry.status.is_posted());

        SearchEntryLinkSearchCriteria {
            link_config_id: self.id.clone(),
            target_side: *entry_link_side,
            target_key,
            hashed_value,
            statuses,
        }
    }

    async fn process_context(
        &self,
        state: &AppState,
        context: EntryLinkContext,
    ) -> LinkProcessingResult<SearchEntryLinkKey> {
        let conn = state.store.get_connection().await.change_context(
            LinkingProcessingError::InternalError("Failed to get database connection".to_string()),
        )?;

        conn.transaction_async(|conn| async move {
            let entry_link_key_new = match context {
                EntryLinkContext::UnLinked { entry_link_key_new } => entry_link_key_new,
                EntryLinkContext::Linked {
                    entry_link_key,
                    new_entry_links,
                    matching_link_key_ids,
                } => {
                    SearchEntryLinkKey::bulk_update_by_ids(
                        &conn,
                        matching_link_key_ids,
                        SearchEntryLinkKeyUpdate::StatusUpdate {
                            status: EntryLinkKeyStatus::Matched,
                        }
                        .into(),
                    )
                    .await
                    .map_err(|e| {
                        logging::error!("Failed to update matching entry link keys: {:?}", e);
                        *e.current_context()
                    })?;

                    EntryLinkNew::bulk_insert_chunked(&conn, new_entry_links)
                        .await
                        .map_err(|e| {
                            logging::error!("Failed to bulk insert entry links: {:?}", e);
                            *e.current_context()
                        })?;

                    entry_link_key
                }
                EntryLinkContext::PostedLinked {
                    entry_link_key,
                    new_entry_link,
                    matched_link_key_id,
                } => {
                    SearchEntryLinkKey::bulk_update_by_ids(
                        &conn,
                        vec![matched_link_key_id],
                        SearchEntryLinkKeyUpdate::StatusUpdate {
                            status: EntryLinkKeyStatus::PostedMatched,
                        }
                        .into(),
                    )
                    .await
                    .map_err(|e| {
                        logging::error!("Failed to update matched posted entry link key: {:?}", e);
                        *e.current_context()
                    })?;

                    new_entry_link.insert(&conn).await.map_err(|e| {
                        logging::error!("Failed to insert posted entry link: {:?}", e);
                        *e.current_context()
                    })?;

                    entry_link_key
                }
            };

            let entry_link_key = entry_link_key_new.insert(&conn).await.map_err(|e| {
                logging::error!("Failed to insert entry link key: {:?}", e);
                *e.current_context()
            })?;

            Entry::update_by_id(
                &conn,
                &entry_link_key.entry_id,
                EntryUpdate::LinkingUpdate {
                    data: EntryLinkingData::V1(EntryLinkingDataV1::Processed),
                }
                .into(),
            )
            .await
            .map_err(|e| {
                logging::error!("Failed to mark entry linking as processed: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(entry_link_key)
        })
        .await
        .change_context(LinkingProcessingError::DatabaseError)
    }
}

fn create_entry_link_key(
    link_config: &LinkConfig,
    domain_entry: &DomainEntry,
    entry_link_side: EntryLinkSide,
    status: EntryLinkKeyStatus,
    encrypted_value: Encryption,
    hashed_value: Secret<String>,
) -> SearchEntryLinkKeyNew {
    let (target_side, target_key, rule_id) = match entry_link_side {
        EntryLinkSide::Transaction => (
            EntryLinkSide::Confirmation,
            link_config.link_search_identifier.get_confirmation_field(),
            &link_config.transaction_rule_id,
        ),
        EntryLinkSide::Confirmation => (
            EntryLinkSide::Transaction,
            link_config.link_search_identifier.get_transaction_field(),
            &link_config.confirmation_rule_id,
        ),
    };

    SearchEntryLinkKeyNew::new(
        link_config.profile_id.clone(),
        domain_entry.account_id.clone(),
        link_config.id.clone(),
        domain_entry.entry_id.clone(),
        rule_id.clone(),
        target_side,
        status,
        target_key,
        encrypted_value,
        hashed_value,
    )
}

fn create_entry_link(
    entry_link_key: &SearchEntryLinkKeyNew,
    matching_entry_link_key: &SearchEntryLinkKey,
    entry_link_side: EntryLinkSide,
    status: EntryLinkStatus,
) -> EntryLinkNew {
    let (transaction_entry_id, confirmation_entry_id) = match entry_link_side {
        EntryLinkSide::Transaction => (
            entry_link_key.entry_id.clone(),
            matching_entry_link_key.entry_id.clone(),
        ),
        EntryLinkSide::Confirmation => (
            matching_entry_link_key.entry_id.clone(),
            entry_link_key.entry_id.clone(),
        ),
    };

    EntryLinkNew::new(
        entry_link_key.profile_id.clone(),
        entry_link_key.account_id.clone(),
        entry_link_key.link_config_id.clone(),
        transaction_entry_id,
        confirmation_entry_id,
        status,
    )
}
