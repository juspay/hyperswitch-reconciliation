use crate::{encryption::ApplicationEncryptionInterface, type_encryption::CryptoOperation};
use common_utils::{
    crypto::Encryptable,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    id_type::MerchantId,
    types::keymanager::Identifier,
};
use error_stack::ResultExt;
use masking::Secret;

/// A utility for streamlined encryption operations that directly produces database-ready encrypted data
pub struct EncryptionHandler<T>
where
    T: ApplicationEncryptionInterface,
{
    pub client: T,
    pub merchant_id: MerchantId,
}

impl<T> EncryptionHandler<T>
where
    T: ApplicationEncryptionInterface,
{
    /// Encrypt a string value directly to Encryption type for database storage
    pub async fn encrypt_string(
        &self,
        value: String,
    ) -> CustomResult<Encryptable<Secret<String>>, ValidationError> {
        let identifier = Identifier::ReconMerchant(self.merchant_id.clone());

        // Encrypt string directly to bytes for database storage
        let encrypted_result = self
            .client
            .crypto_operation(
                CryptoOperation::Encrypt(Secret::<String, masking::WithType>::new(value)),
                identifier,
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Encryption operation failed".to_string(),
            })?;

        encrypted_result
            .try_into_operation()
            .change_context(ValidationError::InvalidValue {
                message: "Failed to extract encrypted data".to_string(),
            })
    }

    /// Encrypt a JSON value directly to Encryption type for database storage
    pub async fn encrypt_json_value(
        &self,
        value: serde_json::Value,
    ) -> CustomResult<Encryptable<Secret<serde_json::Value>>, ValidationError> {
        let identifier = Identifier::ReconMerchant(self.merchant_id.clone());

        // Encrypt JSON string directly to bytes for database storage
        let encrypted_result = self
            .client
            .crypto_operation(
                CryptoOperation::Encrypt(Secret::<serde_json::Value, masking::WithType>::new(
                    value,
                )),
                identifier,
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "JSON encryption operation failed".to_string(),
            })?;

        encrypted_result
            .try_into_operation()
            .change_context(ValidationError::InvalidValue {
                message: "Failed to extract encrypted JSON data".to_string(),
            })
    }

    pub async fn decrypt_json_value(
        &self,
        value: Encryption,
    ) -> CustomResult<Encryptable<Secret<serde_json::Value>>, ValidationError> {
        let identifier = Identifier::ReconMerchant(self.merchant_id.clone());

        let decrypted_result = self
            .client
            .crypto_operation(
                CryptoOperation::<serde_json::Value, masking::WithType>::Decrypt(value),
                identifier,
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "JSON decryption operation failed".to_string(),
            })?;

        decrypted_result
            .try_into_operation()
            .change_context(ValidationError::InvalidValue {
                message: "Failed to extract decrypted JSON data".to_string(),
            })
    }

    /// Encrypt an optional string value
    pub async fn encrypt_optional_string(
        &self,
        value: Option<String>,
    ) -> CustomResult<Option<Encryptable<Secret<String>>>, ValidationError> {
        match value {
            Some(v) => Ok(Some(self.encrypt_string(v).await?)),
            None => Ok(None),
        }
    }

    /// Encrypt an optional JSON value
    pub async fn encrypt_optional_json_value(
        &self,
        value: Option<serde_json::Value>,
    ) -> CustomResult<Option<Encryptable<Secret<serde_json::Value>>>, ValidationError> {
        match value {
            Some(v) => Ok(Some(self.encrypt_json_value(v).await?)),
            None => Ok(None),
        }
    }

    /// Encrypt a batch of JSON values in a single operation
    pub async fn encrypt_batch_json_values(
        &self,
        values: rustc_hash::FxHashMap<String, serde_json::Value>,
    ) -> CustomResult<rustc_hash::FxHashMap<String, Encryption>, ValidationError> {
        if values.is_empty() {
            return Ok(rustc_hash::FxHashMap::default());
        }

        let identifier = Identifier::ReconMerchant(self.merchant_id.clone());

        // Convert to Secret<serde_json::Value> batch
        let secret_values: rustc_hash::FxHashMap<
            String,
            Secret<serde_json::Value, masking::WithType>,
        > = values
            .into_iter()
            .map(|(k, v)| (k, Secret::new(v)))
            .collect();

        // Use TypeEncryption batch operation
        let encrypted_result = self
            .client
            .crypto_operation(CryptoOperation::BatchEncrypt(secret_values), identifier)
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Batch JSON encryption operation failed".to_string(),
            })?;

        let encrypted_batch = encrypted_result.try_into_batchoperation().change_context(
            ValidationError::InvalidValue {
                message: "Failed to extract batch encrypted data".to_string(),
            },
        )?;

        // Convert to Encryption type
        Ok(encrypted_batch
            .into_iter()
            .map(|(k, v)| (k, Encryption::from(v)))
            .collect())
    }

    /// Encrypt a batch of optional JSON values
    pub async fn encrypt_batch_optional_json_values(
        &self,
        values: rustc_hash::FxHashMap<String, Option<serde_json::Value>>,
    ) -> CustomResult<rustc_hash::FxHashMap<String, Option<Encryption>>, ValidationError> {
        let some_values: rustc_hash::FxHashMap<String, serde_json::Value> = values
            .iter()
            .filter_map(|(k, v)| v.as_ref().map(|val| (k.clone(), val.clone())))
            .collect();

        let encrypted_batch = if some_values.is_empty() {
            rustc_hash::FxHashMap::default()
        } else {
            self.encrypt_batch_json_values(some_values).await?
        };

        Ok(values
            .into_iter()
            .map(|(key, optional_value)| {
                let encrypted = optional_value.and_then(|_| encrypted_batch.get(&key).cloned());
                (key, encrypted)
            })
            .collect())
    }
}
