//! Scheduler configuration

use async_trait::async_trait;
use masking::{PeekInterface, Secret};
use recon_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};
use serde::Deserialize;

use crate::scheduler::interfaces::SchedulerError;

/// Configuration for scheduler backends
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "scheduler_backend")]
#[serde(rename_all = "snake_case")]
pub enum SchedulerConfig {
    /// Logs and returns success - local and CI default
    #[default]
    Noop,
    /// Invokr
    Invokr {
        /// Invokr specific configuration
        invokr: InvokrConfig,
    },
}

/// Invokr configuration
#[derive(Debug, Clone, Deserialize)]
pub struct InvokrConfig {
    /// Invokr API base URL, including any path prefix
    pub base_url: String,
    /// Recon's own static API token (an entry in Invokr's `INVOKR_API_STATIC_TOKENS`),
    /// sent as a Bearer token. Not the legacy shared `INVOKR_API_KEY`
    pub api_token: Secret<String>,
    /// Sent as `X-Org-Id`
    pub org_id: String,
    /// Sent as `X-Workspace-Id`
    pub workspace_id: String,
    /// Per-request timeout, defaults to 30s
    pub request_timeout_secs: Option<u64>,
}

impl InvokrConfig {
    /// Validates the Invokr configuration
    pub fn validate(&self) -> error_stack::Result<(), SchedulerError> {
        let invalid = |message: String| {
            error_stack::report!(SchedulerError::ConfigurationError).attach_printable(message)
        };

        url::Url::parse(&self.base_url)
            .map_err(|err| invalid(format!("Invokr base_url is not a valid URL: {err}")))?;
        if self.api_token.peek().trim().is_empty() {
            return Err(invalid("Invokr api_token cannot be empty".to_string()));
        }
        if self.org_id.trim().is_empty() {
            return Err(invalid("Invokr org_id cannot be empty".to_string()));
        }
        if self.workspace_id.trim().is_empty() {
            return Err(invalid("Invokr workspace_id cannot be empty".to_string()));
        }
        Ok(())
    }
}

impl SchedulerConfig {
    /// Validates the scheduler configuration
    pub fn validate(&self) -> error_stack::Result<(), SchedulerError> {
        match self {
            Self::Invokr { invokr } => invokr.validate(),
            Self::Noop => Ok(()),
        }
    }
}

#[async_trait]
impl SecretsHandler for SchedulerConfig {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> error_stack::Result<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        match value.get_inner() {
            Self::Noop => Ok(value.transition_state(|_| Self::Noop)),
            Self::Invokr { invokr } => {
                let api_token = secret_management_client
                    .get_secret(invokr.api_token.clone())
                    .await?;
                let invokr = invokr.clone();

                Ok(value.transition_state(|_| Self::Invokr {
                    invokr: InvokrConfig {
                        api_token: Secret::new(api_token.peek().clone()),
                        ..invokr
                    },
                }))
            }
        }
    }
}
