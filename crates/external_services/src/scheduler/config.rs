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

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "scheduler_backend")]
#[serde(rename_all = "snake_case")]
pub enum SchedulerConfig {
    #[default]
    Noop,
    Invokr {
        invokr: InvokrConfig,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct InvokrConfig {
    pub base_url: String,
    pub api_token: Secret<String>,
    pub org_id: String,
    pub workspace_id: String,
    pub request_timeout_secs: Option<u64>,
}

impl InvokrConfig {
    pub fn validate(&self) -> Result<(), SchedulerError> {
        let invalid = |message: String| {
            error_stack::Report::new(SchedulerError::ConfigurationError).attach(message)
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
    pub fn validate(&self) -> Result<(), SchedulerError> {
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
    ) -> Result<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let Self::Invokr { invokr } = value.get_inner() else {
            return Ok(value.transition_state(|config| config));
        };
        let api_token = secret_management_client
            .get_secret(invokr.api_token.clone())
            .await?;

        Ok(value.transition_state(|config| match config {
            Self::Invokr { invokr } => Self::Invokr {
                invokr: InvokrConfig {
                    api_token,
                    ..invokr
                },
            },
            Self::Noop => Self::Noop,
        }))
    }
}
