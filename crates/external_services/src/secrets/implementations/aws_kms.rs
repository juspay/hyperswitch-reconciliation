//! AWS KMS implementation for secret management

use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_kms::{config::Region, primitives::Blob, Client};
use base64::{engine::general_purpose::STANDARD as BASE64_ENGINE, Engine};
use common_utils::errors::CustomResult;
use error_stack::{Result, ResultExt};
use masking::{PeekInterface, Secret};
use recon_interfaces::secrets_interface::{SecretManagementInterface, SecretsManagementError};
use serde::Deserialize;
/// Configuration parameters required for constructing an [`AwsKmsClient`].
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct AwsKmsConfig {
    /// The AWS key identifier of the KMS key used to encrypt or decrypt data.
    pub key_id: String,

    /// The AWS region to send KMS requests to.
    pub region: String,
}

/// Client for AWS KMS operations.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    inner_client: Client,
    key_id: String,
}

impl AwsKmsClient {
    /// Constructs a new AWS KMS client.
    pub async fn new(config: &AwsKmsConfig) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        Self {
            inner_client: Client::new(&sdk_config),
            key_id: config.key_id.clone(),
        }
    }

    /// Decrypts the provided base64-encoded encrypted data using the AWS KMS SDK.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<String, AwsKmsError> {
        let data = BASE64_ENGINE
            .decode(data)
            .change_context(AwsKmsError::Base64DecodingFailed)?;
        let ciphertext_blob = Blob::new(data);

        let decrypt_output = self
            .inner_client
            .decrypt()
            .key_id(&self.key_id)
            .ciphertext_blob(ciphertext_blob)
            .send()
            .await
            .change_context(AwsKmsError::DecryptionFailed)?;

        let output = decrypt_output
            .plaintext
            .ok_or(AwsKmsError::MissingPlaintextDecryptionOutput)
            .map_err(error_stack::Report::new)
            .and_then(|blob| {
                String::from_utf8(blob.into_inner())
                    .map_err(|_| error_stack::Report::new(AwsKmsError::Utf8DecodingFailed))
            })?;

        Ok(output)
    }

    /// Encrypts the provided String data using the AWS KMS SDK.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> Result<String, AwsKmsError> {
        let plaintext_blob = Blob::new(data.as_ref());

        let encrypted_output = self
            .inner_client
            .encrypt()
            .key_id(&self.key_id)
            .plaintext(plaintext_blob)
            .send()
            .await
            .change_context(AwsKmsError::EncryptionFailed)?;

        let output = encrypted_output
            .ciphertext_blob
            .ok_or(AwsKmsError::MissingCiphertextEncryptionOutput)
            .map(|blob| BASE64_ENGINE.encode(blob.into_inner()))?;

        Ok(output)
    }
}

impl AwsKmsConfig {
    /// Verifies that the [`AwsKmsClient`] configuration is usable.
    pub fn validate(&self) -> Result<(), AwsKmsError> {
        if self.key_id.is_empty() {
            return Err(error_stack::Report::new(AwsKmsError::InvalidConfiguration(
                "KMS AWS key ID must not be empty".to_string(),
            )));
        }

        if self.region.is_empty() {
            return Err(error_stack::Report::new(AwsKmsError::InvalidConfiguration(
                "KMS AWS region must not be empty".to_string(),
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl SecretManagementInterface for AwsKmsClient {
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError> {
        self.decrypt(input.peek())
            .await
            .change_context(SecretsManagementError::FetchSecretFailed)
            .map(Into::into)
    }
}

/// Errors that could occur during KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum AwsKmsError {
    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when AWS KMS decrypting input data.
    #[error("Failed to AWS KMS decrypt input data")]
    DecryptionFailed,

    /// An error occurred when AWS KMS encrypting input data.
    #[error("Failed to AWS KMS encrypt input data")]
    EncryptionFailed,

    /// The AWS KMS decrypted output does not include a plaintext output.
    #[error("Missing plaintext AWS KMS decryption output")]
    MissingPlaintextDecryptionOutput,

    /// The AWS KMS encrypted output does not include a ciphertext output.
    #[error("Missing ciphertext AWS KMS encryption output")]
    MissingCiphertextEncryptionOutput,

    /// An error occurred UTF-8 decoding AWS KMS decrypted output.
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}
