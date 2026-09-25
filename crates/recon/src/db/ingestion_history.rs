use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use common_types::ingestion::ListIngestionHistoryFilters;
use diesel_models::{
    ingestion_history::{
        IngestionConfigHistoryView, IngestionHistory, IngestionHistoryNew, IngestionHistoryUpdate,
        IngestionHistoryUpdateInternal, IngestionHistoryVersionUpdate,
    },
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait IngestionHistoryInterface {
    async fn insert_ingestion_history(
        &self,
        ingestion_history: IngestionHistoryNew,
    ) -> StorageResult<IngestionHistory>;

    async fn find_ingestion_history_by_id_and_profile(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<IngestionHistory>;

    async fn find_config_and_history_by_id_and_profile(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<IngestionConfigHistoryView>;

    async fn list_ingestion_history_by_profile_id(
        &self,
        auth_profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<IngestionHistory>>;

    async fn list_ingestion_history_by_ingestion_id(
        &self,
        ingestion_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<IngestionHistory>>;

    async fn update_ingestion_history_by_id(
        &self,
        id: &str,
        update: IngestionHistoryUpdate,
    ) -> StorageResult<IngestionHistory>;

    async fn list_history_with_config_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListIngestionHistoryFilters,
    ) -> StorageResult<Vec<IngestionConfigHistoryView>>;

    /// Create a new ingestion history version and discard the previous version atomically
    async fn create_ingestion_history_version_and_discard_previous(
        &self,
        previous_version: IngestionHistory,
        version_update: IngestionHistoryVersionUpdate,
    ) -> StorageResult<IngestionHistory>;
}

#[async_trait]
impl IngestionHistoryInterface for Store {
    async fn insert_ingestion_history(
        &self,
        ingestion_history: IngestionHistoryNew,
    ) -> StorageResult<IngestionHistory> {
        let conn = self.get_connection().await?;
        ingestion_history.insert(&conn).await
    }

    async fn find_ingestion_history_by_id_and_profile(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<IngestionHistory> {
        let conn = self.get_connection().await?;
        IngestionHistory::find_by_id_and_profile(&conn, id, profile_id).await
    }

    async fn find_config_and_history_by_id_and_profile(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<IngestionConfigHistoryView> {
        let conn = self.get_connection().await?;
        IngestionHistory::find_config_and_history_by_id_and_profile(&conn, id, profile_id).await
    }

    async fn list_ingestion_history_by_profile_id(
        &self,
        auth_profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<IngestionHistory>> {
        let conn = self.get_connection().await?;
        IngestionHistory::list_by_profile_id(&conn, auth_profile_id, limit).await
    }

    async fn list_ingestion_history_by_ingestion_id(
        &self,
        ingestion_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<IngestionHistory>> {
        let conn = self.get_connection().await?;
        IngestionHistory::list_by_ingestion_id(&conn, ingestion_id, limit).await
    }

    async fn update_ingestion_history_by_id(
        &self,
        id: &str,
        update: IngestionHistoryUpdate,
    ) -> StorageResult<IngestionHistory> {
        let conn = self.get_connection().await?;
        let update_internal = IngestionHistoryUpdateInternal::from(update);
        IngestionHistory::update_by_id(&conn, id, update_internal).await
    }

    async fn list_history_with_config_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListIngestionHistoryFilters,
    ) -> StorageResult<Vec<IngestionConfigHistoryView>> {
        let conn = self.get_connection().await?;
        IngestionHistory::list_config_and_history_by_profile_id(&conn, profile_id, filters).await
    }

    async fn create_ingestion_history_version_and_discard_previous(
        &self,
        previous_version: IngestionHistory,
        version_update: IngestionHistoryVersionUpdate,
    ) -> StorageResult<IngestionHistory> {
        let conn = self.get_connection().await?;

        // Use database transaction to ensure atomicity
        conn.transaction_async(|connection_pool| async move {
            // Step 1: Discard the previous version by updating it
            let discard_update = IngestionHistoryUpdate::DiscardUpdate {
                original_status: previous_version.status,
            };
            let discard_internal = IngestionHistoryUpdateInternal::from(discard_update);

            let _ = IngestionHistory::update_by_id(
                &connection_pool,
                &previous_version.id,
                discard_internal,
            )
            .await
            .map_err(|_| diesel_models::errors::DatabaseError::Others)?;

            // Step 2: Create the new version using the provided previous version
            let new_version =
                IngestionHistoryNew::new_versioned(previous_version, version_update.clone());
            let created_version = new_version
                .insert(&connection_pool)
                .await
                .map_err(|_| diesel_models::errors::DatabaseError::Others)?;

            Ok::<_, diesel_models::errors::DatabaseError>(created_version)
        })
        .await
        .map_err(|e| e.into())
    }
}
