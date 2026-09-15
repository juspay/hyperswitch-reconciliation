use async_trait::async_trait;
use diesel_models::{search_entry_key::SearchEntryKey, StorageResult};

use super::Store;

#[async_trait]
pub trait SearchEntryKeyInterface {
    async fn list_search_entry_keys_by_entry_ids_and_profile_id(
        &self,
        entry_ids: &[String],
        profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<SearchEntryKey>>;
}

#[async_trait]
impl SearchEntryKeyInterface for Store {
    async fn list_search_entry_keys_by_entry_ids_and_profile_id(
        &self,
        entry_ids: &[String],
        profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<SearchEntryKey>> {
        let conn = self.get_connection().await?;
        SearchEntryKey::list_by_entry_ids_and_profile_id(&conn, entry_ids, profile_id, limit).await
    }
}
