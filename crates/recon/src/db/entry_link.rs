use async_trait::async_trait;
use diesel_models::{entry::Entry, entry_link::EntryLink, StorageResult};

use super::Store;

#[async_trait]
pub trait EntryLinkInterface {
    async fn list_linked_entries_by_entry_id_and_profile_id(
        &self,
        entry_id: &str,
        profile_id: &str,
    ) -> StorageResult<Vec<(EntryLink, Entry)>>;
}

#[async_trait]
impl EntryLinkInterface for Store {
    async fn list_linked_entries_by_entry_id_and_profile_id(
        &self,
        entry_id: &str,
        profile_id: &str,
    ) -> StorageResult<Vec<(EntryLink, Entry)>> {
        let conn = self.get_connection().await?;
        EntryLink::list_linked_entries_by_entry_id_and_profile_id(&conn, entry_id, profile_id).await
    }
}
