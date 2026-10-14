use async_trait::async_trait;
use common_enums::EntryLinkSide;
use diesel_models::{
    link_config::{LinkConfig, LinkConfigNew},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait LinkConfigInterface {
    async fn insert_link_config(&self, link_config: LinkConfigNew) -> StorageResult<LinkConfig>;

    async fn list_active_link_configs_by_acc_rule_profile(
        &self,
        account_id: &str,
        rule_id: &str,
        profile_id: &str,
        link_side: &EntryLinkSide,
    ) -> StorageResult<Vec<LinkConfig>>;
}

#[async_trait]
impl LinkConfigInterface for Store {
    async fn insert_link_config(&self, link_config: LinkConfigNew) -> StorageResult<LinkConfig> {
        let conn = self.get_connection().await?;
        link_config.insert(&conn).await
    }

    async fn list_active_link_configs_by_acc_rule_profile(
        &self,
        account_id: &str,
        rule_id: &str,
        profile_id: &str,
        link_side: &EntryLinkSide,
    ) -> StorageResult<Vec<LinkConfig>> {
        let conn = self.get_connection().await?;
        LinkConfig::list_active_by_acc_rule_profile(
            &conn, account_id, rule_id, profile_id, link_side,
        )
        .await
    }
}
