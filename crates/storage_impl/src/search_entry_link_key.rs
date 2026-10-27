use async_trait::async_trait;
use common_enums::{EntryLinkKeyStatus, EntryLinkSide};
use common_types::field_path_key::FieldPathKey;
use diesel_models::{
    search_entry_link_key::{SearchEntryLinkKey, SearchEntryLinkKeyUpdate},
    StorageResult,
};
use masking::{PeekInterface, Secret};

use super::Store;

#[derive(Clone, Debug)]
pub struct SearchEntryLinkSearchCriteria {
    pub link_config_id: String,
    pub target_side: EntryLinkSide,
    pub target_key: FieldPathKey,
    pub hashed_value: Secret<String>,
    pub statuses: Vec<EntryLinkKeyStatus>,
}

#[async_trait]
pub trait SearchEntryLinkKeyInterface {
    async fn bulk_update_search_entry_link_keys_by_ids(
        &self,
        ids: Vec<String>,
        search_entry_link_key_update: SearchEntryLinkKeyUpdate,
    ) -> StorageResult<Vec<SearchEntryLinkKey>>;

    async fn list_search_entry_link_keys_for_linking(
        &self,
        search_criteria: &SearchEntryLinkSearchCriteria,
    ) -> StorageResult<Vec<SearchEntryLinkKey>>;
}

#[async_trait]
impl SearchEntryLinkKeyInterface for Store {
    async fn bulk_update_search_entry_link_keys_by_ids(
        &self,
        ids: Vec<String>,
        search_entry_link_key_update: SearchEntryLinkKeyUpdate,
    ) -> StorageResult<Vec<SearchEntryLinkKey>> {
        let conn = self.get_connection().await?;
        SearchEntryLinkKey::bulk_update_by_ids(&conn, ids, search_entry_link_key_update.into())
            .await
    }

    async fn list_search_entry_link_keys_for_linking(
        &self,
        search_criteria: &SearchEntryLinkSearchCriteria,
    ) -> StorageResult<Vec<SearchEntryLinkKey>> {
        let conn = self.get_connection().await?;
        SearchEntryLinkKey::list_for_linking(
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
