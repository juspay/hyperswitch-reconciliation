use masking::Secret;
use serde::Deserialize;
use std::fmt::Debug;

use crate::hyperswitch::client::HyperswitchClient;

#[derive(Debug, Clone, Deserialize)]
pub struct HyperswitchConfig {
    pub base_url: String,
    pub internal_api_key: Secret<String>,
    pub request_timeout_secs: Option<u64>,
    pub client_idle_timeout: Option<u64>,
}

impl Default for HyperswitchConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            internal_api_key: Secret::new("test_internal_api_key".to_string()),
            request_timeout_secs: Some(30),
            client_idle_timeout: Some(90),
        }
    }
}

impl HyperswitchConfig {
    pub fn get_hyperswitch_client(&self) -> HyperswitchClient {
        HyperswitchClient {
            base_url: self.base_url.clone(),
            internal_api_key: self.internal_api_key.clone(),
            request_timeout_secs: self.request_timeout_secs,
            client_idle_timeout: self.client_idle_timeout,
        }
    }
}
