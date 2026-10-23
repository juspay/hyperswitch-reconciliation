use async_trait::async_trait;
use common_enums::EntryLinkSide;
use diesel_models::{
    link_config::{LinkConfig, LinkConfigNew, LinkConfigUpdate},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait LinkConfigInterface {
    async fn insert_link_config(&self, link_config: LinkConfigNew) -> StorageResult<LinkConfig>;

    async fn list_link_configs_by_profile_id(
        &self,
        profile_id: &str,
    ) -> StorageResult<Vec<LinkConfig>>;

    async fn find_link_config_by_profile_id_and_link_config_id(
        &self,
        profile_id: &str,
        link_config_id: &str,
    ) -> StorageResult<LinkConfig>;

    async fn update_link_config_by_profile_id_and_link_config_id(
        &self,
        profile_id: &str,
        link_config_id: &str,
        update: LinkConfigUpdate,
    ) -> StorageResult<LinkConfig>;

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
    async fn list_link_configs_by_profile_id(
        &self,
        profile_id: &str,
    ) -> StorageResult<Vec<LinkConfig>> {
        let conn = self.get_connection().await?;
        LinkConfig::list_by_profile_id(&conn, profile_id).await
    }

    async fn find_link_config_by_profile_id_and_link_config_id(
        &self,
        profile_id: &str,
        link_config_id: &str,
    ) -> StorageResult<LinkConfig> {
        let conn = self.get_connection().await?;
        LinkConfig::find_by_profile_id_and_link_config_id(&conn, profile_id, link_config_id).await
    }

    async fn update_link_config_by_profile_id_and_link_config_id(
        &self,
        profile_id: &str,
        link_config_id: &str,
        update: LinkConfigUpdate,
    ) -> StorageResult<LinkConfig> {
        let conn = self.get_connection().await?;
        LinkConfig::update_by_profile_id_and_link_config_id(
            &conn,
            profile_id,
            link_config_id,
            update.into(),
        )
        .await
    }

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
