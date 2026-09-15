use async_trait::async_trait;
use common_types::ingestion::ListIngestionConfigsFilters;
use diesel_models::{
    ingestion_config::{
        IngestionConfig, IngestionConfigNew, IngestionConfigUpdate, IngestionConfigUpdateInternal,
    },
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait IngestionConfigInterface {
    async fn insert_ingestion_config(
        &self,
        ingestion_config: IngestionConfigNew,
    ) -> StorageResult<IngestionConfig>;

    async fn find_ingestion_config_by_id(
        &self,
        ingestion_id: &str,
    ) -> StorageResult<IngestionConfig>;

    async fn find_ingestion_config_by_profile_id_and_ingestion_id(
        &self,
        profile_id: &str,
        ingestion_id: &str,
    ) -> StorageResult<IngestionConfig>;

    async fn list_ingestion_configs_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListIngestionConfigsFilters,
    ) -> StorageResult<Vec<IngestionConfig>>;

    async fn update_ingestion_config_by_id(
        &self,
        ingestion_id: &str,
        ingestion_config_update: IngestionConfigUpdate,
    ) -> StorageResult<IngestionConfig>;
}

#[async_trait]
impl IngestionConfigInterface for Store {
    async fn insert_ingestion_config(
        &self,
        ingestion_config: IngestionConfigNew,
    ) -> StorageResult<IngestionConfig> {
        let conn = self.get_connection().await?;
        IngestionConfig::insert(&conn, ingestion_config).await
    }

    async fn find_ingestion_config_by_id(
        &self,
        ingestion_id: &str,
    ) -> StorageResult<IngestionConfig> {
        let conn = self.get_connection().await?;
        IngestionConfig::find_by_ingestion_id(&conn, ingestion_id).await
    }

    async fn find_ingestion_config_by_profile_id_and_ingestion_id(
        &self,
        profile_id: &str,
        ingestion_id: &str,
    ) -> StorageResult<IngestionConfig> {
        let conn = self.get_connection().await?;
        IngestionConfig::find_by_profile_id_and_ingestion_id(&conn, profile_id, ingestion_id).await
    }

    async fn list_ingestion_configs_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListIngestionConfigsFilters,
    ) -> StorageResult<Vec<IngestionConfig>> {
        let conn = self.get_connection().await?;
        IngestionConfig::list_by_profile_id(&conn, profile_id, filters).await
    }

    async fn update_ingestion_config_by_id(
        &self,
        ingestion_id: &str,
        ingestion_config_update: IngestionConfigUpdate,
    ) -> StorageResult<IngestionConfig> {
        let conn = self.get_connection().await?;
        let update_internal = IngestionConfigUpdateInternal::from(ingestion_config_update);

        IngestionConfig::update_by_ingestion_id(&conn, ingestion_id, update_internal).await
    }
}
