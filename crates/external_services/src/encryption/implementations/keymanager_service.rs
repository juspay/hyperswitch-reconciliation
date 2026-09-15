//! Keymanager service encryption implementation

use async_trait::async_trait;
use common_utils::{
    crypto,
    errors::CustomResult,
    keymanager,
    types::keymanager::{EncryptionCreateRequest, Identifier, KeyManagerState},
};
use error_stack::ResultExt;
use masking::Secret;
use recon_domain_models::{
    type_encryption::{encrypt::TypeEncryption, CryptoOperation, CryptoOutput},
    ApplicationEncryptionError, ApplicationEncryptionInterface,
};

/// Keymanager service encryption client that delegates all operations to the external keymanager
#[derive(Clone)]
pub struct KeymanagerServiceEncryption {
    keymanager_state: KeyManagerState,
}

impl KeymanagerServiceEncryption {
    /// Creates a new KeymanagerServiceEncryption instance
    pub fn new(keymanager_state: KeyManagerState) -> Self {
        Self { keymanager_state }
    }
}

#[async_trait]
impl ApplicationEncryptionInterface for KeymanagerServiceEncryption {
    /// Creates a key in the external keymanager for the given identifier
    async fn setup_key_for_identifier(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<(), ApplicationEncryptionError> {
        keymanager::create_key_in_key_manager(
            &self.keymanager_state,
            EncryptionCreateRequest {
                identifier: identifier.clone(),
            },
        )
        .await
        .change_context(ApplicationEncryptionError::EncryptionFailed)
        .attach_printable("Failed to create key in KeyManager")?;

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
        match operation {
            CryptoOperation::Encrypt(data) => {
                let encrypted =
                    crypto::Encryptable::encrypt_via_api(&self.keymanager_state, data, identifier)
                        .await
                        .change_context(ApplicationEncryptionError::EncryptionFailed)?;
                Ok(CryptoOutput::Operation(encrypted))
            }
            CryptoOperation::EncryptOptional(data) => match data {
                Some(data) => {
                    let encrypted = crypto::Encryptable::encrypt_via_api(
                        &self.keymanager_state,
                        data,
                        identifier,
                    )
                    .await
                    .change_context(ApplicationEncryptionError::EncryptionFailed)?;
                    Ok(CryptoOutput::OptionalOperation(Some(encrypted)))
                }
                None => Ok(CryptoOutput::OptionalOperation(None)),
            },
            CryptoOperation::Decrypt(data) => {
                let decrypted =
                    crypto::Encryptable::decrypt_via_api(&self.keymanager_state, data, identifier)
                        .await
                        .change_context(ApplicationEncryptionError::DecryptionFailed)?;
                Ok(CryptoOutput::Operation(decrypted))
            }
            CryptoOperation::DecryptOptional(data) => match data {
                Some(data) => {
                    let decrypted = crypto::Encryptable::decrypt_via_api(
                        &self.keymanager_state,
                        data,
                        identifier,
                    )
                    .await
                    .change_context(ApplicationEncryptionError::DecryptionFailed)?;
                    Ok(CryptoOutput::OptionalOperation(Some(decrypted)))
                }
                None => Ok(CryptoOutput::OptionalOperation(None)),
            },
            CryptoOperation::BatchEncrypt(data) => {
                let encrypted = crypto::Encryptable::batch_encrypt_chunked_via_api(
                    &self.keymanager_state,
                    data,
                    identifier,
                )
                .await
                .change_context(ApplicationEncryptionError::EncryptionFailed)?;
                Ok(CryptoOutput::BatchOperation(encrypted))
            }
            CryptoOperation::BatchDecrypt(data) => {
                let decrypted = crypto::Encryptable::batch_decrypt_chunked_via_api(
                    &self.keymanager_state,
                    data,
                    identifier,
                )
                .await
                .change_context(ApplicationEncryptionError::DecryptionFailed)?;
                Ok(CryptoOutput::BatchOperation(decrypted))
            }
        }
    }
}
