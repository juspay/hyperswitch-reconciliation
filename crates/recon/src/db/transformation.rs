use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use common_types::transformation::{
    ListTransformationConfigsFilters, ListTransformationHistoryFilters,
};
use diesel_models::{
    errors::DatabaseError,
    metadata_schema::MetadataSchemaNew,
    transformation_config::{
        TransformationConfig, TransformationConfigNew, TransformationConfigUpdate,
    },
    transformation_history::{
        TransformationConfigHistoryView, TransformationHistory, TransformationHistoryNew,
        TransformationHistoryUpdate,
    },
    StorageResult,
};
use error_stack::ResultExt;

use super::Store;

#[async_trait]
pub trait TransformationConfigInterface {
    async fn insert_transformation_config(
        &self,
        transformation_config: TransformationConfigNew,
    ) -> StorageResult<TransformationConfig>;

    async fn find_transformation_config_by_id_and_profile_id(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<TransformationConfig>;

    async fn list_transformation_configs_by_ids_and_profile_id(
        &self,
        ids: Vec<String>,
        profile_id: &str,
    ) -> StorageResult<Vec<TransformationConfig>>;

    async fn list_transformation_configs_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListTransformationConfigsFilters,
    ) -> StorageResult<Vec<TransformationConfig>>;

    // Keep this for internal use by execute_transformation
    async fn list_transformation_configs_by_ingestion_id(
        &self,
        ingestion_id: &str,
        is_active: Option<bool>,
    ) -> StorageResult<Vec<TransformationConfig>>;

    // Create transformation config with metadata schema in transaction
    async fn insert_transformation_config_with_metadata_schema(
        &self,
        transformation_config: TransformationConfigNew,
        metadata_schema: MetadataSchemaNew,
    ) -> StorageResult<TransformationConfig>;

    async fn update_transformation_config_by_id(
        &self,
        id: &str,
        update: TransformationConfigUpdate,
    ) -> StorageResult<TransformationConfig>;
}

#[async_trait]
pub trait TransformationHistoryInterface {
    async fn insert_transformation_history(
        &self,
        transformation_history: TransformationHistoryNew,
    ) -> StorageResult<TransformationHistory>;

    async fn find_config_and_history_by_id(
        &self,
        id: &str,
    ) -> StorageResult<TransformationConfigHistoryView>;

    async fn update_transformation_history_by_id(
        &self,
        id: &str,
        update: TransformationHistoryUpdate,
    ) -> StorageResult<TransformationHistory>;

    async fn list_history_with_config_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListTransformationHistoryFilters,
    ) -> StorageResult<Vec<TransformationConfigHistoryView>>;

    async fn list_transformation_history_by_transformation_id(
        &self,
        transformation_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<TransformationHistory>>;
}

#[async_trait]
impl TransformationConfigInterface for Store {
    async fn insert_transformation_config(
        &self,
        transformation_config: TransformationConfigNew,
    ) -> StorageResult<TransformationConfig> {
        let conn = self.get_connection().await?;
        transformation_config.insert(&conn).await
    }

    async fn find_transformation_config_by_id_and_profile_id(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<TransformationConfig> {
        let conn = self.get_connection().await?;
        TransformationConfig::find_by_id_and_profile_id(&conn, id, profile_id).await
    }

    async fn list_transformation_configs_by_ids_and_profile_id(
        &self,
        ids: Vec<String>,
        profile_id: &str,
    ) -> StorageResult<Vec<TransformationConfig>> {
        let conn = self.get_connection().await?;
        TransformationConfig::list_by_ids_and_profile_id(&conn, ids, profile_id).await
    }

    async fn list_transformation_configs_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListTransformationConfigsFilters,
    ) -> StorageResult<Vec<TransformationConfig>> {
        let conn = self.get_connection().await?;
        TransformationConfig::list_by_profile_id(&conn, profile_id, filters).await
    }

    async fn list_transformation_configs_by_ingestion_id(
        &self,
        ingestion_id: &str,
        is_active: Option<bool>,
    ) -> StorageResult<Vec<TransformationConfig>> {
        let conn = self.get_connection().await?;
        TransformationConfig::list_by_ingestion_id(&conn, ingestion_id, is_active).await
    }

    async fn insert_transformation_config_with_metadata_schema(
        &self,
        transformation_config: TransformationConfigNew,
        metadata_schema: MetadataSchemaNew,
    ) -> StorageResult<TransformationConfig> {
        // Capture IDs for error context before moving structs
        let transformation_id = transformation_config.id.clone();
        let schema_id = metadata_schema.id.clone();

        let conn = self.get_connection().await?;

        conn.transaction_async(|connection_pool| async move {
            // Create the metadata schema first
            metadata_schema
                .insert(&connection_pool)
                .await
                .map_err(|e| *e.current_context())?;

            // Create the transformation config with the metadata schema ID
            let created_transformation_config = transformation_config
                .insert(&connection_pool)
                .await
                .map_err(|e| *e.current_context())?;

            Ok::<_, DatabaseError>(created_transformation_config)
        })
        .await
        .map_err(|e| error_stack::report!(e))
        .attach_printable_lazy(|| {
            format!(
                "Failed to create transformation config '{transformation_id}' with metadata schema '{schema_id}' atomically"
            )
        })
    }

    async fn update_transformation_config_by_id(
        &self,
        id: &str,
        update: TransformationConfigUpdate,
    ) -> StorageResult<TransformationConfig> {
        let conn = self.get_connection().await?;
        TransformationConfig::update_by_id(&conn, id, update.into()).await
    }
}

#[async_trait]
impl TransformationHistoryInterface for Store {
    async fn insert_transformation_history(
        &self,
        transformation_history: TransformationHistoryNew,
    ) -> StorageResult<TransformationHistory> {
        let conn = self.get_connection().await?;
        transformation_history.insert(&conn).await
    }

    async fn find_config_and_history_by_id(
        &self,
        id: &str,
    ) -> StorageResult<TransformationConfigHistoryView> {
        let conn = self.get_connection().await?;
        TransformationHistory::find_config_and_history_by_id(&conn, id).await
    }

    async fn update_transformation_history_by_id(
        &self,
        id: &str,
        update: TransformationHistoryUpdate,
    ) -> StorageResult<TransformationHistory> {
        let conn = self.get_connection().await?;
        TransformationHistory::update_by_id(&conn, id, update.into()).await
    }

    async fn list_history_with_config_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListTransformationHistoryFilters,
    ) -> StorageResult<Vec<TransformationConfigHistoryView>> {
        let conn = self.get_connection().await?;
        TransformationHistory::list_history_with_config_by_profile_id(&conn, profile_id, filters)
            .await
    }

    async fn list_transformation_history_by_transformation_id(
        &self,
        transformation_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<TransformationHistory>> {
        let conn = self.get_connection().await?;
        TransformationHistory::list_by_transformation_id(&conn, transformation_id, limit).await
    }
}
