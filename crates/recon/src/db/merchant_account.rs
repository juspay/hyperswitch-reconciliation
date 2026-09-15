use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use diesel_models::{
    merchant_account::{MerchantAccount, MerchantAccountNew},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait MerchantAccountInterface {
    async fn insert_merchant_account(
        &self,
        merchant_account: MerchantAccountNew,
    ) -> StorageResult<MerchantAccount>;

    async fn find_merchant_account_by_id(
        &self,
        merchant_id: &MerchantId,
    ) -> StorageResult<MerchantAccount>;
}

#[async_trait]
impl MerchantAccountInterface for Store {
    async fn insert_merchant_account(
        &self,
        merchant_account: MerchantAccountNew,
    ) -> StorageResult<MerchantAccount> {
        let conn = self.get_connection().await?;

        merchant_account.insert(conn.as_async_conn()).await
    }

    async fn find_merchant_account_by_id(
        &self,
        merchant_id: &MerchantId,
    ) -> StorageResult<MerchantAccount> {
        let conn = self.get_connection().await?;

        MerchantAccount::find_by_merchant_id(conn.as_async_conn(), merchant_id).await
    }
}
