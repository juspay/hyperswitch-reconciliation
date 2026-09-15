//! Application encryption module
//!
//! This module provides interfaces and implementations for application data encryption
//! with support for local encryption using master keys and keymanager service.

use async_trait::async_trait;
use common_utils::{errors::CustomResult, types::keymanager::Identifier};
use masking::Secret;

use crate::type_encryption::{CryptoOperation, CryptoOutput};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ApplicationEncryptionError {
    #[error("Invalid configuration: {message}")]
    InvalidConfiguration { message: String },
    #[error("Encryption operation failed")]
    EncryptionFailed,
    #[error("Decryption operation failed")]
    DecryptionFailed,
}

/// Application encryption interface that mirrors the existing crypto_operation function
#[async_trait]
pub trait ApplicationEncryptionInterface: Send + Sync {
    /// Creates/ensures a key exists for the given identifier
    /// For LocalEncryption: No-op (keys are derived on demand)
    /// For KeymanagerService: Creates key in external keymanager
    async fn setup_key_for_identifier(
        &self,
        identifier: &Identifier,
    ) -> CustomResult<(), ApplicationEncryptionError>;

    async fn crypto_operation<T: Clone + Send, S: masking::Strategy<T> + Send>(
        &self,
        operation: CryptoOperation<T, S>,
        identifier: Identifier,
    ) -> CustomResult<CryptoOutput<T, S>, ApplicationEncryptionError>
    where
        Secret<T, S>: Send,
        common_utils::crypto::Encryptable<Secret<T, S>>:
            crate::type_encryption::encrypt::TypeEncryption<T, common_utils::crypto::GcmAes256, S>;
}
