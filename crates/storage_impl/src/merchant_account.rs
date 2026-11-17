use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use diesel_models::{
    merchant_account::{MerchantAccount, MerchantAccountNew},
    StorageResult,
};

use crate::database::connection::{DatabaseRead, DatabaseWrite};

#[async_trait]
pub trait MerchantAccountReadInterface {
    async fn find_merchant_account_by_id(
        &self,
        merchant_id: &MerchantId,
    ) -> StorageResult<MerchantAccount>;
}

#[async_trait]
pub trait MerchantAccountWriteInterface {
    async fn insert_merchant_account(
        &self,
        merchant_account: MerchantAccountNew,
    ) -> StorageResult<MerchantAccount>;
}

#[async_trait]
impl<T: DatabaseRead> MerchantAccountReadInterface for T {
    async fn find_merchant_account_by_id(
        &self,
        merchant_id: &MerchantId,
    ) -> StorageResult<MerchantAccount> {
        let conn = self.read_conn().await?;

        MerchantAccount::find_by_merchant_id(&conn, merchant_id).await
    }
}

#[async_trait]
impl<T: DatabaseWrite> MerchantAccountWriteInterface for T {
    async fn insert_merchant_account(
        &self,
        merchant_account: MerchantAccountNew,
    ) -> StorageResult<MerchantAccount> {
        let conn = self.write_conn().await?;

        merchant_account.insert(&conn).await
    }
}
