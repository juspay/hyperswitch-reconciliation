use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use diesel_models::{
    api_key::{ApiKey, ApiKeyNew},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait ApiKeyInterface {
    async fn insert_api_key(&self, api_key: ApiKeyNew) -> StorageResult<ApiKey>;

    async fn find_active_api_key_by_merchant_id_and_id(
        &self,
        merchant_id: &MerchantId,
        api_key_id: &str,
    ) -> StorageResult<ApiKey>;

    /// Newest first.
    async fn list_active_api_keys_by_merchant_id(
        &self,
        merchant_id: &MerchantId,
    ) -> StorageResult<Vec<ApiKey>>;
}

#[async_trait]
impl ApiKeyInterface for Store {
    async fn insert_api_key(&self, api_key: ApiKeyNew) -> StorageResult<ApiKey> {
        let conn = self.get_connection().await?;

        api_key.insert(conn.as_async_conn()).await
    }

    async fn find_active_api_key_by_merchant_id_and_id(
        &self,
        merchant_id: &MerchantId,
        api_key_id: &str,
    ) -> StorageResult<ApiKey> {
        let conn = self.get_connection().await?;

        ApiKey::find_active_by_merchant_id_and_api_key_id(
            conn.as_async_conn(),
            merchant_id,
            api_key_id,
        )
        .await
    }

    async fn list_active_api_keys_by_merchant_id(
        &self,
        merchant_id: &MerchantId,
    ) -> StorageResult<Vec<ApiKey>> {
        let conn = self.get_connection().await?;

        ApiKey::list_active_by_merchant_id(conn.as_async_conn(), merchant_id).await
    }
}
