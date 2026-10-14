//! OCI Vault KMS secret management.

use async_trait::async_trait;
use error_stack::{Report, Result, ResultExt};
use masking::{PeekInterface, Secret};
pub use oci_kms::OciKmsConfig;

use common_utils::errors::CustomResult;
use recon_interfaces::secrets_interface::{SecretManagementInterface, SecretsManagementError};

/// OCI Vault KMS client.
#[derive(Clone)]
pub struct OciKmsClient {
    inner_client: oci_kms::OciKmsClient,
}

impl OciKmsClient {
    /// Creates a client; credentials are resolved on first use.
    pub fn new(config: &OciKmsConfig) -> Result<Self, OciKmsError> {
        let inner_client = oci_kms::OciKmsClient::new(config)
            .map_err(Report::new)
            .change_context(OciKmsError::ClientCreationFailed)?;
        Ok(Self { inner_client })
    }

    /// Decrypts ciphertext to a UTF-8 secret.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<String, OciKmsError> {
        let ciphertext =
            std::str::from_utf8(data.as_ref()).change_context(OciKmsError::Utf8DecodingFailed)?;
        let plaintext = self
            .inner_client
            .decrypt(ciphertext)
            .await
            .map_err(Report::new)
            .change_context(OciKmsError::DecryptionFailed)?;
        String::from_utf8(plaintext).change_context(OciKmsError::Utf8DecodingFailed)
    }

    /// Encrypts a configuration secret.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> Result<String, OciKmsError> {
        self.inner_client
            .encrypt(data.as_ref())
            .await
            .map_err(Report::new)
            .change_context(OciKmsError::EncryptionFailed)
    }
}

#[async_trait]
impl SecretManagementInterface for OciKmsClient {
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError> {
        self.decrypt(input.peek())
            .await
            .change_context(SecretsManagementError::FetchSecretFailed)
            .map(Secret::new)
    }
}

/// OCI KMS errors.
#[derive(Debug, thiserror::Error)]
pub enum OciKmsError {
    #[error("Failed to create OCI KMS client")]
    ClientCreationFailed,
    #[error("Failed to OCI KMS decrypt input data")]
    DecryptionFailed,
    #[error("Failed to OCI KMS encrypt input data")]
    EncryptionFailed,
    #[error("Failed to UTF-8 decode OCI KMS input or output")]
    Utf8DecodingFailed,
}
