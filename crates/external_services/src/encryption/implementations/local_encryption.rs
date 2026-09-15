//! Local encryption implementation using master key

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_ENGINE, Engine};
use common_utils::{crypto, errors::CustomResult, types::keymanager::Identifier};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use recon_domain_models::{
    type_encryption::{encrypt::TypeEncryption, CryptoOperation, CryptoOutput},
    ApplicationEncryptionError, ApplicationEncryptionInterface,
};
use sha2::{Digest, Sha256};

/// Local encryption client that uses a master key for all operations
#[derive(Clone)]
pub struct LocalEncryption {
    master_key: Vec<u8>,
}

impl LocalEncryption {
    /// Creates a new LocalEncryption instance with the provided master key
    pub fn new(master_key: Secret<String>) -> CustomResult<Self, ApplicationEncryptionError> {
        // Decode base64 master key
        let key_bytes = BASE64_ENGINE.decode(master_key.peek()).change_context(
            ApplicationEncryptionError::InvalidConfiguration {
                message: "Invalid base64 master key".to_string(),
            },
        )?;

        Ok(Self {
            master_key: key_bytes,
        })
    }

    /// Derives a key for a specific identifier using SHA256(master_key + identifier)
    fn derive_key_for_identifier(&self, identifier: &Identifier) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.master_key);
        match identifier {
            Identifier::ReconMerchant(merchant_id) => {
                hasher.update(merchant_id.get_string_repr().as_bytes());
            }
        }
        hasher.finalize().to_vec()
    }
}

#[async_trait]
impl ApplicationEncryptionInterface for LocalEncryption {
    /// For LocalEncryption, keys are derived on-demand, so this is a no-op
    async fn setup_key_for_identifier(
        &self,
        _identifier: &Identifier,
    ) -> CustomResult<(), ApplicationEncryptionError> {
        // No-op for local encryption since keys are derived on demand
        Ok(())
    }

    async fn crypto_operation<T: Clone + Send, S: masking::Strategy<T> + Send>(
        &self,
        operation: CryptoOperation<T, S>,
        identifier: Identifier,
    ) -> CustomResult<CryptoOutput<T, S>, ApplicationEncryptionError>
    where
        Secret<T, S>: Send,
        crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
    {
        let derived_key = self.derive_key_for_identifier(&identifier);

        match operation {
            CryptoOperation::Encrypt(data) => {
                let encrypted = crypto::Encryptable::encrypt(data, &derived_key, crypto::GcmAes256)
                    .await
                    .change_context(ApplicationEncryptionError::EncryptionFailed)?;
                Ok(CryptoOutput::Operation(encrypted))
            }
            CryptoOperation::EncryptOptional(data) => match data {
                Some(data) => {
                    let encrypted =
                        crypto::Encryptable::encrypt(data, &derived_key, crypto::GcmAes256)
                            .await
                            .change_context(ApplicationEncryptionError::EncryptionFailed)?;
                    Ok(CryptoOutput::OptionalOperation(Some(encrypted)))
                }
                None => Ok(CryptoOutput::OptionalOperation(None)),
            },
            CryptoOperation::Decrypt(data) => {
                let decrypted = crypto::Encryptable::decrypt(data, &derived_key, crypto::GcmAes256)
                    .await
                    .change_context(ApplicationEncryptionError::DecryptionFailed)?;
                Ok(CryptoOutput::Operation(decrypted))
            }
            CryptoOperation::DecryptOptional(data) => match data {
                Some(data) => {
                    let decrypted =
                        crypto::Encryptable::decrypt(data, &derived_key, crypto::GcmAes256)
                            .await
                            .change_context(ApplicationEncryptionError::DecryptionFailed)?;
                    Ok(CryptoOutput::OptionalOperation(Some(decrypted)))
                }
                None => Ok(CryptoOutput::OptionalOperation(None)),
            },
            CryptoOperation::BatchEncrypt(data) => {
                let encrypted =
                    crypto::Encryptable::batch_encrypt(data, &derived_key, crypto::GcmAes256)
                        .await
                        .change_context(ApplicationEncryptionError::EncryptionFailed)?;
                Ok(CryptoOutput::BatchOperation(encrypted))
            }
            CryptoOperation::BatchDecrypt(data) => {
                let decrypted =
                    crypto::Encryptable::batch_decrypt(data, &derived_key, crypto::GcmAes256)
                        .await
                        .change_context(ApplicationEncryptionError::DecryptionFailed)?;
                Ok(CryptoOutput::BatchOperation(decrypted))
            }
        }
    }
}
