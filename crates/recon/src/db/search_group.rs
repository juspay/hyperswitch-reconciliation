use async_trait::async_trait;
use diesel_models::{
    search_group::{SearchGroup, SearchGroupNew, SearchGroupUpdate},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait SearchGroupInterface {
    async fn insert_search_group(&self, search_group: SearchGroupNew)
        -> StorageResult<SearchGroup>;

    async fn batch_insert_search_groups(
        &self,
        search_groups: Vec<SearchGroupNew>,
    ) -> StorageResult<Vec<SearchGroup>>;

    async fn find_by_id(&self, id: &str) -> StorageResult<SearchGroup>;

    async fn find_by_id_and_profile_id(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<SearchGroup>;

    async fn find_by_rule_id_and_hashed_group_value(
        &self,
        rule_id: &str,
        hashed_grouping_value: &str,
    ) -> StorageResult<Option<SearchGroup>>;

    async fn find_by_primary_transaction_id_and_profile_id(
        &self,
        primary_transaction_id: &str,
        profile_id: &str,
    ) -> StorageResult<Option<SearchGroup>>;

    async fn discard_by_id(
        &self,
        id: &str,
        update: SearchGroupUpdate,
    ) -> StorageResult<SearchGroup>;
}

#[async_trait]
impl SearchGroupInterface for Store {
    async fn insert_search_group(
        &self,
        search_group: SearchGroupNew,
    ) -> StorageResult<SearchGroup> {
        let conn = self.get_connection().await?;
        search_group.insert(&conn).await
    }

    async fn batch_insert_search_groups(
        &self,
        search_groups: Vec<SearchGroupNew>,
    ) -> StorageResult<Vec<SearchGroup>> {
        let conn = self.get_connection().await?;
        SearchGroupNew::batch_insert(search_groups, &conn).await
    }

    async fn find_by_id(&self, id: &str) -> StorageResult<SearchGroup> {
        let conn = self.get_connection().await?;
        SearchGroup::find_by_id(&conn, id.to_string()).await
    }

    async fn find_by_id_and_profile_id(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<SearchGroup> {
        let conn = self.get_connection().await?;
        SearchGroup::find_by_id_and_profile_id(&conn, id, profile_id).await
    }

    async fn find_by_rule_id_and_hashed_group_value(
        &self,
        rule_id: &str,
        hashed_grouping_value: &str,
    ) -> StorageResult<Option<SearchGroup>> {
        let conn = self.get_connection().await?;
        SearchGroup::find_by_rule_id_and_hashed_group_value(&conn, rule_id, hashed_grouping_value)
            .await
    }

    async fn find_by_primary_transaction_id_and_profile_id(
        &self,
        primary_transaction_id: &str,
        profile_id: &str,
    ) -> StorageResult<Option<SearchGroup>> {
        let conn = self.get_connection().await?;
        SearchGroup::find_by_primary_transaction_id_and_profile_id(
            &conn,
            primary_transaction_id,
            profile_id,
        )
        .await
    }

    async fn discard_by_id(
        &self,
        id: &str,
        update: SearchGroupUpdate,
    ) -> StorageResult<SearchGroup> {
        let conn = self.get_connection().await?;
        let update_internal = update.into();
        SearchGroup::discard_by_id(&conn, id, update_internal).await
    }
}
