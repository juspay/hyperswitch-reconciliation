//! No encryption implementation for development/testing

use async_trait::async_trait;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use recon_interfaces::secrets_interface::{SecretManagementInterface, SecretsManagementError};

/// No encryption type - passes through values unchanged
#[derive(Debug, Clone)]
pub struct NoEncryption;

impl NoEncryption {
    /// Create a new NoEncryption instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoEncryption {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretManagementInterface for NoEncryption {
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError> {
        // For no encryption, we just return the input as-is
        // This handles environment variable substitution if needed
        let raw = input.clone().expose();
        if raw.starts_with("${") && raw.ends_with('}') {
            let env_var = &raw[2..raw.len() - 1];
            let value = std::env::var(env_var)
                .map_err(|_| SecretsManagementError::FetchSecretFailed)
                .attach_printable_lazy(|| format!("Failed to read env var: {env_var}"))?;
            Ok(Secret::new(value))
        } else {
            Ok(input)
        }
    }
}
