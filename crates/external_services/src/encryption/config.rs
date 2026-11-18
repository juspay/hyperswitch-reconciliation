//! Application encryption configuration and client factory

use crate::encryption::implementations::{
    keymanager_service::KeymanagerServiceEncryption, local_encryption::LocalEncryption,
};
use async_trait::async_trait;
use base64::Engine;
use common_utils::{
    consts::{BASE64_ENGINE, DEFAULT_TENANT_ID},
    errors::{CustomResult, KeyManagerClientError},
    id_type::MerchantId,
    types::keymanager::{Identifier, KeyManagerState},
};
use masking::{PeekInterface, Secret};
use recon_domain_models::{
    type_encryption::{CryptoOperation, CryptoOutput},
    ApplicationEncryptionError, ApplicationEncryptionInterface, EncryptionHandler,
};
use recon_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};
use serde::Deserialize;
use std::result::Result as StdResult;

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;
/// Configuration for local encryption
#[derive(Debug, Clone, Deserialize)]
pub struct LocalEncryptionConfig {
    /// Base64 encoded master key
    pub master_key: Secret<String>,
}

/// Configuration for key manager encryption
#[derive(Debug, Clone, Deserialize)]
pub struct KeyManagerConfig {
    pub enabled: bool,
    pub url: String,
    pub cert: Secret<String>,
    pub ca: Secret<String>,
    pub client_idle_timeout: Option<u64>,
}

impl Default for KeyManagerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            url: "https://localhost:5000".to_string(),
            cert: Secret::new("".to_string()),
            ca: Secret::new("".to_string()),
            client_idle_timeout: None,
        }
    }
}

/// Enum representing configuration options for application encryption.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "encryption_mode")]
#[serde(rename_all = "snake_case")]
pub enum ApplicationEncryptionConfig {
    /// Local encryption using master key from configuration
    LocalEncryption {
        /// Local encryption config
        local_encryption: LocalEncryptionConfig,
    },

    /// External keymanager service encryption
    KeyManager {
        /// Key manager config
        key_manager: KeyManagerConfig,
    },
}

impl Default for ApplicationEncryptionConfig {
    fn default() -> Self {
        Self::LocalEncryption {
            local_encryption: LocalEncryptionConfig {
                master_key: Secret::new("base64-encoded-key".to_string()),
            },
        }
    }
}

/// Concrete encryption client that wraps the different implementations
#[derive(Clone)]
pub enum ApplicationEncryptionClient {
    Local(LocalEncryption),
    KeymanagerService(KeymanagerServiceEncryption),
}

#[async_trait]
impl ApplicationEncryptionInterface for ApplicationEncryptionClient {
    async fn setup_key_for_identifier(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<(), ApplicationEncryptionError> {
        match self {
            Self::Local(client) => client.setup_key_for_identifier(identifier).await,
            Self::KeymanagerService(client) => client.setup_key_for_identifier(identifier).await,
        }
    }

    async fn crypto_operation<T: Clone + Send, S: masking::Strategy<T> + Send>(
        &self,
        operation: CryptoOperation<T, S>,
        identifier: Identifier,
    ) -> CustomResult<CryptoOutput<T, S>, ApplicationEncryptionError>
    where
        Secret<T, S>: Send,
        common_utils::crypto::Encryptable<Secret<T, S>>:
            recon_domain_models::type_encryption::encrypt::TypeEncryption<
                T,
                common_utils::crypto::GcmAes256,
                S,
            >,
    {
        match self {
            Self::Local(client) => client.crypto_operation(operation, identifier).await,
            Self::KeymanagerService(client) => client.crypto_operation(operation, identifier).await,
        }
    }
}

impl ApplicationEncryptionClient {
    pub async fn health_check(&self) -> CustomResult<(), KeyManagerClientError> {
        match self {
            Self::Local(_) => Ok(()),
            Self::KeymanagerService(client) => client.health_check().await,
        }
    }

    pub fn get_encryptor_for_merchant(
        &self,
        merchant_id: MerchantId,
    ) -> EncryptionHandler<impl ApplicationEncryptionInterface> {
        EncryptionHandler {
            client: self.clone(),
            merchant_id,
        }
    }
}

impl ApplicationEncryptionConfig {
    /// Creates the appropriate encryption client based on the configuration
    pub async fn get_encryption_client(
        &self,
    ) -> Result<ApplicationEncryptionClient, ApplicationEncryptionError> {
        match self {
            Self::LocalEncryption { local_encryption } => {
                let client = LocalEncryption::new(local_encryption.master_key.clone())?;
                Ok(ApplicationEncryptionClient::Local(client))
            }
            Self::KeyManager { key_manager } => {
                // Create KeyManagerState from embedded config
                let keymanager_state = KeyManagerState {
                    tenant_id: DEFAULT_TENANT_ID.to_string(),
                    enabled: key_manager.enabled,
                    url: key_manager.url.clone(),
                    cert: key_manager.cert.clone(),
                    ca: key_manager.ca.clone(),
                    client_idle_timeout: key_manager.client_idle_timeout,
                };

                let client = KeymanagerServiceEncryption::new(keymanager_state);
                Ok(ApplicationEncryptionClient::KeymanagerService(client))
            }
        }
    }

    /// Validates the encryption configuration
    pub fn validate(&self) -> StdResult<(), String> {
        match self {
            Self::LocalEncryption { local_encryption } => {
                let key_str = local_encryption.master_key.peek().trim();

                // Check if master key is empty
                if key_str.is_empty() {
                    return Err("Master key cannot be empty for LocalEncryption variant".into());
                }

                // Validate base64 format
                match BASE64_ENGINE.decode(key_str) {
                    Ok(decoded) => {
                        // Check key length (should be 32 bytes for AES-256)
                        if decoded.len() != 32 {
                            return Err(format!(
                                "Master key must be exactly 32 bytes when decoded, got {} bytes",
                                decoded.len()
                            ));
                        }

                        // Check if key is all zeros (weak key)
                        if decoded.iter().all(|&b| b == 0) {
                            return Err("Master key cannot be all zeros (weak key)".into());
                        }
                    }
                    Err(_) => {
                        return Err("Master key must be valid base64 encoded string".into());
                    }
                }

                Ok(())
            }
            Self::KeyManager { key_manager } => {
                // Validate KeyManager configuration
                if !key_manager.enabled {
                    return Err("KeyManager must be enabled for KeyManager variant".into());
                }

                if key_manager.url.trim().is_empty() {
                    return Err("KeyManager URL cannot be empty for KeyManager variant".into());
                }

                if key_manager.cert.peek().trim().is_empty()
                    || key_manager.ca.peek().trim().is_empty()
                {
                    return Err(
                        "KeyManager certificates (cert and ca) are required for KeyManager variant"
                            .into(),
                    );
                }

                Ok(())
            }
        }
    }
}

#[async_trait]
impl SecretsHandler for ApplicationEncryptionConfig {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> Result<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let config = value.get_inner();

        match config {
            Self::LocalEncryption { local_encryption } => {
                // Decrypt the master key using KMS
                let decrypted_master_key = secret_management_client
                    .get_secret(local_encryption.master_key.clone())
                    .await?;

                Ok(value.transition_state(|_| Self::LocalEncryption {
                    local_encryption: LocalEncryptionConfig {
                        master_key: Secret::new(decrypted_master_key.peek().clone()),
                    },
                }))
            }
            Self::KeyManager { key_manager } => {
                // Store the values we need before moving
                let enabled = key_manager.enabled;
                let url = key_manager.url.clone();
                let client_idle_timeout = key_manager.client_idle_timeout;

                // Decrypt the certificates using KMS
                let decrypted_cert = secret_management_client
                    .get_secret(key_manager.cert.clone())
                    .await?;
                let decrypted_ca = secret_management_client
                    .get_secret(key_manager.ca.clone())
                    .await?;

                Ok(value.transition_state(|_| Self::KeyManager {
                    key_manager: KeyManagerConfig {
                        enabled,
                        url,
                        cert: Secret::new(decrypted_cert.peek().clone()),
                        ca: Secret::new(decrypted_ca.peek().clone()),
                        client_idle_timeout,
                    },
                }))
            }
        }
    }
}
