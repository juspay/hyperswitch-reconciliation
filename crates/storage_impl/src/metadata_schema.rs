use async_trait::async_trait;
use common_types::metadata_schema::ListMetadataSchemasFilters;
use diesel_models::{
    metadata_schema::{
        MetadataSchema, MetadataSchemaNew, MetadataSchemaUpdate, MetadataSchemaVersionUpdate,
    },
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait MetadataSchemaInterface {
    async fn insert_metadata_schema(
        &self,
        metadata_schema: MetadataSchemaNew,
    ) -> StorageResult<MetadataSchema>;

    async fn find_metadata_schema_by_id_and_profile_id(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<MetadataSchema>;

    async fn list_metadata_schemas_by_ids_and_profile_id(
        &self,
        ids: Vec<String>,
        profile_id: &str,
    ) -> StorageResult<Vec<MetadataSchema>>;

    async fn list_metadata_schemas_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListMetadataSchemasFilters,
    ) -> StorageResult<Vec<MetadataSchema>>;

    /// Create a new metadata schema version with incremented version number
    async fn create_new_metadata_schema_version(
        &self,
        previous_version: MetadataSchema,
        version_update: MetadataSchemaVersionUpdate,
    ) -> StorageResult<MetadataSchema>;

    async fn update_metadata_schema_by_id(
        &self,
        id: &str,
        update: MetadataSchemaUpdate,
    ) -> StorageResult<MetadataSchema>;
}

#[async_trait]
impl MetadataSchemaInterface for Store {
    async fn insert_metadata_schema(
        &self,
        metadata_schema: MetadataSchemaNew,
    ) -> StorageResult<MetadataSchema> {
        let conn = self.get_connection().await?;
        metadata_schema.insert(&conn).await
    }

    async fn find_metadata_schema_by_id_and_profile_id(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<MetadataSchema> {
        let conn = self.get_connection().await?;
        MetadataSchema::find_by_id_and_profile_id(&conn, id, profile_id).await
    }

    async fn list_metadata_schemas_by_ids_and_profile_id(
        &self,
        ids: Vec<String>,
        profile_id: &str,
    ) -> StorageResult<Vec<MetadataSchema>> {
        let conn = self.get_connection().await?;
        MetadataSchema::list_by_ids_and_profile_id(&conn, ids, profile_id).await
    }

    async fn list_metadata_schemas_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListMetadataSchemasFilters,
    ) -> StorageResult<Vec<MetadataSchema>> {
        let conn = self.get_connection().await?;
        MetadataSchema::list_by_profile_id(&conn, profile_id, filters).await
    }

    async fn create_new_metadata_schema_version(
        &self,
        previous_version: MetadataSchema,
        version_update: MetadataSchemaVersionUpdate,
    ) -> StorageResult<MetadataSchema> {
        let conn = self.get_connection().await?;
        let new_version = MetadataSchemaNew::new_versioned(previous_version, version_update);
        new_version.insert(&conn).await
    }

    async fn update_metadata_schema_by_id(
        &self,
        id: &str,
        update: MetadataSchemaUpdate,
    ) -> StorageResult<MetadataSchema> {
        let conn = self.get_connection().await?;
        MetadataSchema::update_by_id(&conn, id, update.into()).await
    }
}
