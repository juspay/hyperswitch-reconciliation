use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use diesel_models::{
    errors::DatabaseError,
    recon_rule::{ReconRule, ReconRuleNew, ReconRuleUpdate},
    recon_rule_account::{ReconRuleAccount, ReconRuleAccountNew},
    StorageResult,
};
use error_stack::ResultExt;

use super::Store;

#[async_trait]
pub trait ReconRuleInterface {
    async fn insert_recon_rule(&self, recon_rule: ReconRuleNew) -> StorageResult<ReconRule>;
    async fn find_recon_rule_by_id(&self, rule_id: &str) -> StorageResult<ReconRule>;
    async fn find_recon_rule_by_profile_id_and_rule_id(
        &self,
        profile_id: &str,
        rule_id: &str,
    ) -> StorageResult<ReconRule>;
    async fn list_recon_rules_by_profile_id(
        &self,
        profile_id: &str,
        is_active: Option<bool>,
    ) -> StorageResult<Vec<ReconRule>>;
    async fn create_recon_rule_with_sources_and_targets(
        &self,
        recon_rule: ReconRuleNew,
        accounts: Vec<ReconRuleAccountNew>,
    ) -> StorageResult<(ReconRule, Vec<ReconRuleAccount>)>;
    async fn list_recon_rules_by_ids(
        &self,
        profile_id: &str,
        rule_ids: Vec<String>,
    ) -> StorageResult<Vec<ReconRule>>;
    async fn update_recon_rule_by_id(
        &self,
        rule_id: &str,
        recon_rule_update: ReconRuleUpdate,
    ) -> StorageResult<ReconRule>;
}

#[async_trait]
impl ReconRuleInterface for Store {
    async fn insert_recon_rule(&self, recon_rule: ReconRuleNew) -> StorageResult<ReconRule> {
        let conn = self.get_connection().await?;
        recon_rule.insert(&conn).await
    }

    async fn find_recon_rule_by_id(&self, rule_id: &str) -> StorageResult<ReconRule> {
        let conn = self.get_connection().await?;
        ReconRule::find_by_rule_id(&conn, rule_id).await
    }

    async fn find_recon_rule_by_profile_id_and_rule_id(
        &self,
        profile_id: &str,
        rule_id: &str,
    ) -> StorageResult<ReconRule> {
        let conn = self.get_connection().await?;
        ReconRule::find_by_profile_id_and_rule_id(&conn, profile_id, rule_id).await
    }

    async fn list_recon_rules_by_profile_id(
        &self,
        profile_id: &str,
        is_active: Option<bool>,
    ) -> StorageResult<Vec<ReconRule>> {
        let conn = self.get_connection().await?;
        ReconRule::list_by_profile_id(&conn, profile_id, is_active).await
    }

    async fn create_recon_rule_with_sources_and_targets(
        &self,
        recon_rule: ReconRuleNew,
        accounts: Vec<ReconRuleAccountNew>,
    ) -> StorageResult<(ReconRule, Vec<ReconRuleAccount>)> {
        let conn = self.get_connection().await?;

        conn.transaction_async(|connection_pool| async move {
            // Insert the main recon rule
            let created_rule = recon_rule
                .insert(&connection_pool)
                .await
                .map_err(|_| DatabaseError::Others)?;

            // Insert all accounts
            let mut created_accounts = Vec::new();
            for account in accounts {
                let created_account = account
                    .insert(&connection_pool)
                    .await
                    .map_err(|_| DatabaseError::Others)?;
                created_accounts.push(created_account);
            }

            Ok::<_, DatabaseError>((created_rule, created_accounts))
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable("Failed to create recon rule with sources and targets in transaction")
    }

    async fn list_recon_rules_by_ids(
        &self,
        profile_id: &str,
        rule_ids: Vec<String>,
    ) -> StorageResult<Vec<ReconRule>> {
        let conn = self.get_connection().await?;
        ReconRule::list_by_rule_ids(&conn, profile_id, rule_ids).await
    }

    async fn update_recon_rule_by_id(
        &self,
        rule_id: &str,
        recon_rule_update: ReconRuleUpdate,
    ) -> StorageResult<ReconRule> {
        let conn = self.get_connection().await?;
        ReconRule::update_by_rule_id(&conn, rule_id, recon_rule_update.into()).await
    }
}
