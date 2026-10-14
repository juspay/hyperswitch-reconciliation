use async_trait::async_trait;
use common_enums::{EntryLinkKeyStatus, EntryLinkSide};
use common_types::field_path_key::FieldPathKey;
use diesel_models::{
    entry_link_key::{EntryLinkKey, EntryLinkKeyUpdate},
    StorageResult,
};
use masking::{PeekInterface, Secret};

use super::Store;

#[derive(Clone, Debug)]
pub struct EntryLinkSearchCriteria {
    pub link_config_id: String,
    pub target_side: EntryLinkSide,
    pub target_key: FieldPathKey,
    pub hashed_value: Secret<String>,
    pub statuses: Vec<EntryLinkKeyStatus>,
}

#[async_trait]
pub trait EntryLinkKeyInterface {
    async fn bulk_update_entry_link_keys_by_ids(
        &self,
        ids: Vec<String>,
        entry_link_key_update: EntryLinkKeyUpdate,
    ) -> StorageResult<Vec<EntryLinkKey>>;

    async fn list_entry_link_keys_for_linking(
        &self,
        search_criteria: &EntryLinkSearchCriteria,
    ) -> StorageResult<Vec<EntryLinkKey>>;
}

#[async_trait]
impl EntryLinkKeyInterface for Store {
    async fn bulk_update_entry_link_keys_by_ids(
        &self,
        ids: Vec<String>,
        entry_link_key_update: EntryLinkKeyUpdate,
    ) -> StorageResult<Vec<EntryLinkKey>> {
        let conn = self.get_connection().await?;
        EntryLinkKey::bulk_update_by_ids(&conn, ids, entry_link_key_update.into()).await
    }

    async fn list_entry_link_keys_for_linking(
        &self,
        search_criteria: &EntryLinkSearchCriteria,
    ) -> StorageResult<Vec<EntryLinkKey>> {
        let conn = self.get_connection().await?;
        EntryLinkKey::list_for_linking(
            &conn,
            &search_criteria.link_config_id,
            &search_criteria.target_side,
            &search_criteria.target_key,
            search_criteria.hashed_value.peek(),
            search_criteria.statuses.to_vec(),
        )
        .await
    }
}
