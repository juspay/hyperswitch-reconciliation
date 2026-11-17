use async_trait::async_trait;
use common_enums::ReconRuleAccountType;
use diesel_models::{
    recon_rule_account::{ReconRuleAccount, ReconRuleAccountNew},
    StorageResult,
};

use crate::database::connection::{DatabaseRead, DatabaseWrite};

#[async_trait]
pub trait ReconRuleAccountReadInterface {
    async fn list_recon_rule_accounts_by_account_id_and_type(
        &self,
        account_id: &str,
        account_type: ReconRuleAccountType,
    ) -> StorageResult<Vec<ReconRuleAccount>>;
}

#[async_trait]
pub trait ReconRuleAccountWriteInterface {
    async fn insert_recon_rule_account(
        &self,
        recon_rule_account: ReconRuleAccountNew,
    ) -> StorageResult<ReconRuleAccount>;
}

#[async_trait]
impl<T: DatabaseRead> ReconRuleAccountReadInterface for T {
    async fn list_recon_rule_accounts_by_account_id_and_type(
        &self,
        account_id: &str,
        account_type: ReconRuleAccountType,
    ) -> StorageResult<Vec<ReconRuleAccount>> {
        let conn = self.read_conn().await?;
        ReconRuleAccount::list_by_account_id_and_account_type(&conn, account_id, account_type).await
    }
}

#[async_trait]
impl<T: DatabaseWrite> ReconRuleAccountWriteInterface for T {
    async fn insert_recon_rule_account(
        &self,
        recon_rule_account: ReconRuleAccountNew,
    ) -> StorageResult<ReconRuleAccount> {
        let conn = self.write_conn().await?;
        recon_rule_account.insert(&conn).await
    }
}
