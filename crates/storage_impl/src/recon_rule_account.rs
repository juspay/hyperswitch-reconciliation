use async_trait::async_trait;
use common_enums::ReconRuleAccountType;
use diesel_models::{
    recon_rule_account::{ReconRuleAccount, ReconRuleAccountNew},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait ReconRuleAccountInterface {
    async fn insert_recon_rule_account(
        &self,
        recon_rule_account: ReconRuleAccountNew,
    ) -> StorageResult<ReconRuleAccount>;

    async fn list_recon_rule_accounts_by_account_id_and_type(
        &self,
        account_id: &str,
        account_type: ReconRuleAccountType,
    ) -> StorageResult<Vec<ReconRuleAccount>>;
}

#[async_trait]
impl ReconRuleAccountInterface for Store {
    async fn insert_recon_rule_account(
        &self,
        recon_rule_account: ReconRuleAccountNew,
    ) -> StorageResult<ReconRuleAccount> {
        let conn = self.get_connection().await?;
        recon_rule_account.insert(&conn).await
    }

    async fn list_recon_rule_accounts_by_account_id_and_type(
        &self,
        account_id: &str,
        account_type: ReconRuleAccountType,
    ) -> StorageResult<Vec<ReconRuleAccount>> {
        let conn = self.get_connection().await?;
        ReconRuleAccount::list_by_account_id_and_account_type(&conn, account_id, account_type).await
    }
}
