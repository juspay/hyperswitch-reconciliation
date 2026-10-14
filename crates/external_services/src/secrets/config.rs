//! Secrets management configuration and client factory

use std::sync::Arc;

use error_stack::{Result, ResultExt};
use serde::Deserialize;

use recon_interfaces::secrets_interface::{SecretManagementInterface, SecretsManagementError};

use crate::secrets::implementations::{
    aws_kms::{AwsKmsClient, AwsKmsConfig},
    no_encryption::NoEncryption,
    oci_kms::{OciKmsClient, OciKmsConfig},
};

/// Enum representing configuration options for secrets management.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(tag = "secrets_manager")]
#[serde(rename_all = "snake_case")]
pub enum SecretsManagementConfig {
    /// AWS KMS configuration
    AwsKms {
        /// AWS KMS config
        aws_kms: AwsKmsConfig,
    },

    /// OCI Vault KMS configuration
    OciKms {
        /// OCI crypto endpoint and key
        oci_kms: OciKmsConfig,
    },

    /// Variant representing no encryption
    #[default]
    NoEncryption,
}

impl SecretsManagementConfig {
    /// Verifies that the client configuration is usable
    pub fn validate(&self) -> Result<(), SecretsManagementError> {
        match self {
            Self::AwsKms { aws_kms } => aws_kms
                .validate()
                .map_err(|_| SecretsManagementError::InvalidConfiguration {
                    message: "AWS KMS configuration invalid".to_string(),
                })
                .map_err(error_stack::Report::new),
            Self::OciKms { oci_kms } => oci_kms
                .validate()
                .map_err(|_| SecretsManagementError::InvalidConfiguration {
                    message: "OCI KMS configuration invalid".to_string(),
                })
                .map_err(error_stack::Report::new),
            Self::NoEncryption => Ok(()),
        }
    }

    /// Retrieves the appropriate secret management client based on the configuration.
    pub async fn get_secret_management_client(
        &self,
    ) -> Result<Arc<dyn SecretManagementInterface>, SecretsManagementError> {
        match self {
            Self::AwsKms { aws_kms } => {
                let client = AwsKmsClient::new(aws_kms).await;
                Ok(Arc::new(client))
            }
            Self::OciKms { oci_kms } => {
                let client = OciKmsClient::new(oci_kms)
                    .change_context(SecretsManagementError::ClientCreationFailed)?;
                Ok(Arc::new(client))
            }
            Self::NoEncryption => Ok(Arc::new(NoEncryption)),
        }
    }
}
