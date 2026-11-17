use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use diesel_models::{
    api_key::{ApiKey, ApiKeyNew, ApiKeyUpdate},
    StorageResult,
};

use crate::database::connection::{DatabaseRead, DatabaseWrite};

#[async_trait]
pub trait ApiKeyReadInterface {
    async fn find_active_api_key_by_merchant_id_and_id(
        &self,
        merchant_id: &MerchantId,
        api_key_id: &str,
    ) -> StorageResult<ApiKey>;

    /// Authentication lookup. Not scoped by merchant, so the caller must compare `merchant_id`.
    async fn find_active_api_key_by_hash(&self, hashed_api_key: &str) -> StorageResult<ApiKey>;

    /// Newest first.
    async fn list_active_api_keys_by_merchant_id(
        &self,
        merchant_id: &MerchantId,
    ) -> StorageResult<Vec<ApiKey>>;
}

#[async_trait]
pub trait ApiKeyWriteInterface {
    async fn insert_api_key(&self, api_key: ApiKeyNew) -> StorageResult<ApiKey>;

    async fn update_api_key(
        &self,
        merchant_id: &MerchantId,
        api_key_id: &str,
        update: ApiKeyUpdate,
    ) -> StorageResult<ApiKey>;
}

#[async_trait]
impl<T: DatabaseRead> ApiKeyReadInterface for T {
    async fn find_active_api_key_by_merchant_id_and_id(
        &self,
        merchant_id: &MerchantId,
        api_key_id: &str,
    ) -> StorageResult<ApiKey> {
        let conn = self.read_conn().await?;

        ApiKey::find_active_by_merchant_id_and_api_key_id(&conn, merchant_id, api_key_id).await
    }

    async fn find_active_api_key_by_hash(&self, hashed_api_key: &str) -> StorageResult<ApiKey> {
        let conn = self.read_conn().await?;

        ApiKey::find_active_by_hashed_api_key(&conn, hashed_api_key).await
    }

    async fn list_active_api_keys_by_merchant_id(
        &self,
        merchant_id: &MerchantId,
    ) -> StorageResult<Vec<ApiKey>> {
        let conn = self.read_conn().await?;

        ApiKey::list_active_by_merchant_id(&conn, merchant_id).await
    }
}

#[async_trait]
impl<T: DatabaseWrite> ApiKeyWriteInterface for T {
    async fn insert_api_key(&self, api_key: ApiKeyNew) -> StorageResult<ApiKey> {
        let conn = self.write_conn().await?;

        api_key.insert(&conn).await
    }

    async fn update_api_key(
        &self,
        merchant_id: &MerchantId,
        api_key_id: &str,
        update: ApiKeyUpdate,
    ) -> StorageResult<ApiKey> {
        let conn = self.write_conn().await?;

        ApiKey::update_active_by_merchant_id_and_api_key_id(
            &conn,
            merchant_id,
            api_key_id,
            update.into(),
        )
        .await
    }
}
