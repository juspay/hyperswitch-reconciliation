use serde::Deserialize;

use crate::{
    user::client::{mock_user_client::MockUserClient, user::UserClient},
    HyperswitchConfig,
};

/// Enum representing configuration options for HyperSwitch user client.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "client_mode")]
#[serde(rename_all = "snake_case")]
pub enum UserConfig {
    /// Local service that returns mock data (for testing)
    #[default]
    MockUserClient,

    /// External HyperSwitch service (actual HTTP calls)
    Hyperswitch {
        hyperswitch_config: HyperswitchConfig,
    },
}

impl UserConfig {
    pub fn get_user_client(&self) -> UserClient {
        match self {
            Self::MockUserClient => UserClient::MockUserClient(MockUserClient),
            Self::Hyperswitch { hyperswitch_config } => {
                UserClient::Hyperswitch(hyperswitch_config.get_hyperswitch_client())
            }
        }
    }
}
