use async_bb8_diesel::AsyncConnection;
use common_enums::{EntryLinkKeyStatus, EntryLinkSide, EntryLinkStatus};
use common_types::entry_linking::{EntryLinkingData, EntryLinkingDataV1};
use common_utils::{encryption::Encryption, logging};
use diesel_models::{
    entry::{Entry, EntryUpdate},
    entry_link::EntryLinkNew,
    entry_link_key::{EntryLinkKey, EntryLinkKeyNew, EntryLinkKeyUpdate},
    errors::DatabaseError,
    link_config::LinkConfig,
};
use error_stack::ResultExt;
use masking::Secret;
use recon_domain_models::{entry::DomainEntry, transaction::DomainTransaction};

use crate::{
    db::entry_link_key::{EntryLinkKeyInterface, EntryLinkSearchCriteria},
    domain::linking_engine::{
        EntryLinkContext, LinkProcessingResult, LinkingEngine, LinkingProcessingError,
    },
    routes::AppState,
    utils::recon_engine::hash_string,
};

use self::link_matching_key::LinkSearchIdentifierExt;

pub mod link_matching_key;
pub mod link_trigger;

impl LinkingEngine for LinkConfig {
    async fn create_context(
        &self,
        state: &AppState,
        domain_entry: DomainEntry,
        _domain_transaction: DomainTransaction,
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
            .list_entry_link_keys_for_linking(&search_criteria)
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
        matching_entry_link_keys: Vec<EntryLinkKey>,
        encrypted_value: Encryption,
        hashed_value: Secret<String>,
    ) -> LinkProcessingResult<EntryLinkContext> {
        let (target_side, target_key, rule_id) = match entry_link_side {
            EntryLinkSide::Transaction => (
                EntryLinkSide::Confirmation,
                self.link_search_identifier.get_confirmation_field(),
                &self.transaction_rule_id,
            ),
            EntryLinkSide::Confirmation => (
                EntryLinkSide::Transaction,
                self.link_search_identifier.get_transaction_field(),
                &self.confirmation_rule_id,
            ),
        };

        let unlinked_posted_keys = matching_entry_link_keys
            .into_iter()
            .filter(|key| key.status == EntryLinkKeyStatus::UnlinkedPosted)
            .collect::<Vec<_>>();

        match unlinked_posted_keys.as_slice() {
            [] => {
                let entry_link_key_new = EntryLinkKeyNew::new(
                    self.profile_id.clone(),
                    domain_entry.account_id.clone(),
                    self.id.clone(),
                    domain_entry.entry_id.clone(),
                    rule_id.clone(),
                    target_side,
                    EntryLinkKeyStatus::UnlinkedPosted,
                    target_key,
                    encrypted_value,
                    hashed_value,
                );

                Ok(EntryLinkContext::UnLinked { entry_link_key_new })
            }
            [matched_entry_link_key] => {
                let entry_link_key_new = EntryLinkKeyNew::new(
                    self.profile_id.clone(),
                    domain_entry.account_id.clone(),
                    self.id.clone(),
                    domain_entry.entry_id.clone(),
                    rule_id.clone(),
                    target_side,
                    EntryLinkKeyStatus::LinkedPosted,
                    target_key,
                    encrypted_value,
                    hashed_value,
                );

                let (transaction_entry_id, confirmation_entry_id) = match entry_link_side {
                    EntryLinkSide::Transaction => (
                        domain_entry.entry_id.clone(),
                        matched_entry_link_key.entry_id.clone(),
                    ),
                    EntryLinkSide::Confirmation => (
                        matched_entry_link_key.entry_id.clone(),
                        domain_entry.entry_id.clone(),
                    ),
                };

                let entry_link_new = EntryLinkNew::new(
                    self.profile_id.clone(),
                    domain_entry.account_id.clone(),
                    self.id.clone(),
                    transaction_entry_id,
                    confirmation_entry_id,
                    entry_link_key_new.encrypted_value.clone(),
                    entry_link_key_new.hashed_value.clone(),
                    EntryLinkStatus::PostedLink,
                );

                Ok(EntryLinkContext::PostedLinked {
                    entry_link_key: entry_link_key_new,
                    entry_link_new,
                    matched_link_key_id: matched_entry_link_key.id.clone(),
                })
            }
            _ => Err(LinkingProcessingError::MultiplePostedLinksFound.into()),
        }
    }

    fn create_non_posted_context(
        &self,
        domain_entry: &DomainEntry,
        entry_link_side: EntryLinkSide,
        matching_link_keys: Vec<EntryLinkKey>,
        encrypted_value: Encryption,
        hashed_value: Secret<String>,
    ) -> LinkProcessingResult<EntryLinkContext> {
        let (target_side, target_key, rule_id) = match entry_link_side {
            EntryLinkSide::Transaction => (
                EntryLinkSide::Confirmation,
                self.link_search_identifier.get_confirmation_field(),
                &self.transaction_rule_id,
            ),
            EntryLinkSide::Confirmation => (
                EntryLinkSide::Transaction,
                self.link_search_identifier.get_transaction_field(),
                &self.confirmation_rule_id,
            ),
        };
        let status = if matching_link_keys.is_empty() {
            EntryLinkKeyStatus::Unlinked
        } else {
            EntryLinkKeyStatus::Linked
        };

        let entry_link_key_new = EntryLinkKeyNew::new(
            self.profile_id.clone(),
            domain_entry.account_id.clone(),
            self.id.clone(),
            domain_entry.entry_id.clone(),
            rule_id.clone(),
            target_side,
            status,
            target_key,
            encrypted_value,
            hashed_value,
        );

        if matching_link_keys.is_empty() {
            return Ok(EntryLinkContext::UnLinked { entry_link_key_new });
        }

        let entry_link_news = matching_link_keys
            .iter()
            .map(|matching_entry_link_key| {
                let (transaction_entry_id, confirmation_entry_id) = match entry_link_side {
                    EntryLinkSide::Transaction => (
                        domain_entry.entry_id.clone(),
                        matching_entry_link_key.entry_id.clone(),
                    ),
                    EntryLinkSide::Confirmation => (
                        matching_entry_link_key.entry_id.clone(),
                        domain_entry.entry_id.clone(),
                    ),
                };

                EntryLinkNew::new(
                    self.profile_id.clone(),
                    domain_entry.account_id.clone(),
                    self.id.clone(),
                    transaction_entry_id,
                    confirmation_entry_id,
                    entry_link_key_new.encrypted_value.clone(),
                    entry_link_key_new.hashed_value.clone(),
                    EntryLinkStatus::Linked,
                )
            })
            .collect();

        Ok(EntryLinkContext::Linked {
            entry_link_key: entry_link_key_new,
            entry_link_news,
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
    ) -> EntryLinkSearchCriteria {
        let target_key = match entry_link_side {
            EntryLinkSide::Transaction => self.link_search_identifier.get_transaction_field(),
            EntryLinkSide::Confirmation => self.link_search_identifier.get_confirmation_field(),
        };

        let statuses = EntryLinkKeyStatus::from_posted_status(domain_entry.status.is_posted());

        EntryLinkSearchCriteria {
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
    ) -> LinkProcessingResult<EntryLinkKey> {
        let conn = state.store.get_connection().await.change_context(
            LinkingProcessingError::InternalError("Failed to get database connection".to_string()),
        )?;

        conn.transaction_async(|conn| async move {
            let entry_link_key_new = match context {
                EntryLinkContext::UnLinked { entry_link_key_new } => entry_link_key_new,
                EntryLinkContext::Linked {
                    entry_link_key,
                    entry_link_news,
                    matching_link_key_ids,
                } => {
                    EntryLinkKey::bulk_update_by_ids(
                        &conn,
                        matching_link_key_ids,
                        EntryLinkKeyUpdate::StatusUpdate {
                            status: EntryLinkKeyStatus::Linked,
                        }
                        .into(),
                    )
                    .await
                    .map_err(|e| {
                        logging::error!("Failed to update matching entry link keys: {:?}", e);
                        *e.current_context()
                    })?;

                    EntryLinkNew::bulk_insert_chunked(&conn, entry_link_news)
                        .await
                        .map_err(|e| {
                            logging::error!("Failed to bulk insert entry links: {:?}", e);
                            *e.current_context()
                        })?;

                    entry_link_key
                }
                EntryLinkContext::PostedLinked {
                    entry_link_key,
                    entry_link_new,
                    matched_link_key_id,
                } => {
                    EntryLinkKey::bulk_update_by_ids(
                        &conn,
                        vec![matched_link_key_id],
                        EntryLinkKeyUpdate::StatusUpdate {
                            status: EntryLinkKeyStatus::LinkedPosted,
                        }
                        .into(),
                    )
                    .await
                    .map_err(|e| {
                        logging::error!("Failed to update matched posted entry link key: {:?}", e);
                        *e.current_context()
                    })?;

                    entry_link_new.insert(&conn).await.map_err(|e| {
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
