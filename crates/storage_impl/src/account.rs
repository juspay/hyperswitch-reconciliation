use async_trait::async_trait;
use diesel_models::{
    account::{Account, AccountNew, AccountUpdate},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait AccountInterface {
    async fn insert_account(&self, account: AccountNew) -> StorageResult<Account>;
    async fn find_account_by_id(&self, account_id: &str) -> StorageResult<Account>;
    async fn find_account_by_profile_id_and_account_id(
        &self,
        profile_id: &str,
        account_id: &str,
    ) -> StorageResult<Account>;
    async fn list_accounts_by_profile_id(&self, profile_id: &str) -> StorageResult<Vec<Account>>;
    async fn list_accounts_by_profile_id_and_account_ids(
        &self,
        profile_id: &str,
        account_ids: &[String],
    ) -> StorageResult<Vec<Account>>;
    async fn update_account_by_id(
        &self,
        account_id: &str,
        account_update: AccountUpdate,
    ) -> StorageResult<Account>;
}

#[async_trait]
impl AccountInterface for Store {
    async fn insert_account(&self, account: AccountNew) -> StorageResult<Account> {
        let conn = self.get_connection().await?;
        account.insert(&conn).await
    }

    async fn find_account_by_id(&self, account_id: &str) -> StorageResult<Account> {
        let conn = self.get_connection().await?;
        Account::find_by_account_id(&conn, account_id).await
    }

    async fn find_account_by_profile_id_and_account_id(
        &self,
        profile_id: &str,
        account_id: &str,
    ) -> StorageResult<Account> {
        let conn = self.get_connection().await?;
        Account::find_by_profile_id_and_account_id(&conn, profile_id, account_id).await
    }
    async fn list_accounts_by_profile_id(&self, profile_id: &str) -> StorageResult<Vec<Account>> {
        let conn = self.get_connection().await?;
        Account::list_by_profile_id(&conn, profile_id).await
    }

    async fn list_accounts_by_profile_id_and_account_ids(
        &self,
        profile_id: &str,
        account_ids: &[String],
    ) -> StorageResult<Vec<Account>> {
        let conn = self.get_connection().await?;
        Account::find_by_profile_id_and_account_ids(&conn, profile_id, account_ids).await
    }

    async fn update_account_by_id(
        &self,
        account_id: &str,
        account_update: AccountUpdate,
    ) -> StorageResult<Account> {
        let conn = self.get_connection().await?;
        let account_update_internal = account_update.into();
        Account::update_by_account_id(&conn, account_id, account_update_internal).await
    }
}
