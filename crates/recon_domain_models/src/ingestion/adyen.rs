use common_utils::{
    crypto::EncryptableName,
    errors::{CustomResult, ValidationError},
    types::keymanager::Identifier,
};
use diesel_models::ingestion_config::adyen::AdyenIngestionConfig as DieselAdyenIngestionConfig;
use error_stack::ResultExt;
use serde::Serialize;

use crate::{
    behaviour::Conversion, encryption_handler::EncryptionHandler, type_encryption::CryptoOperation,
    ApplicationEncryptionInterface,
};

#[derive(Clone, Debug, Serialize)]
pub struct AdyenIngestionConfig {
    pub hmac_secret: EncryptableName,
    pub webhook_basic_auth: AdyenBasicAuth,
    pub report_basic_auth: AdyenBasicAuth,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdyenBasicAuth {
    pub username: String,
    pub password: EncryptableName,
}

#[async_trait::async_trait]
impl Conversion for AdyenIngestionConfig {
    type DstType = DieselAdyenIngestionConfig;
    type NewDstType = DieselAdyenIngestionConfig;

    async fn convert_back(
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let decrypted_hmac = encryptor
            .client
            .crypto_operation(
                CryptoOperation::Decrypt(item.hmac_secret.clone()),
                Identifier::ReconMerchant(encryptor.merchant_id.clone()),
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt hmac_secret".to_string(),
            })?
            .try_into_operation()
            .change_context(ValidationError::InvalidValue {
                message: "Failed to convert decrypted hmac_secret".to_string(),
            })?;

        // Decrypt webhook password
        let decrypted_webhook_password = encryptor
            .client
            .crypto_operation(
                CryptoOperation::Decrypt(item.webhook_basic_auth.password.clone()),
                Identifier::ReconMerchant(encryptor.merchant_id.clone()),
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt webhook password".to_string(),
            })?
            .try_into_operation()
            .change_context(ValidationError::InvalidValue {
                message: "Failed to convert decrypted webhook password".to_string(),
            })?;

        // Decrypt report password
        let decrypted_report_password = encryptor
            .client
            .crypto_operation(
                CryptoOperation::Decrypt(item.report_basic_auth.password.clone()),
                Identifier::ReconMerchant(encryptor.merchant_id.clone()),
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt report password".to_string(),
            })?
            .try_into_operation()
            .change_context(ValidationError::InvalidValue {
                message: "Failed to convert decrypted report password".to_string(),
            })?;

        Ok(Self {
            hmac_secret: decrypted_hmac,
            webhook_basic_auth: AdyenBasicAuth {
                username: item.webhook_basic_auth.username,
                password: decrypted_webhook_password,
            },
            report_basic_auth: AdyenBasicAuth {
                username: item.report_basic_auth.username,
                password: decrypted_report_password,
            },
        })
    }

    fn construct_new(self) -> Self::NewDstType {
        use diesel_models::ingestion_config::adyen::{
            AdyenBasicAuth as DieselAdyenBasicAuth,
            AdyenIngestionConfig as DieselAdyenIngestionConfig,
        };

        DieselAdyenIngestionConfig {
            hmac_secret: self.hmac_secret.into(),
            webhook_basic_auth: DieselAdyenBasicAuth {
                username: self.webhook_basic_auth.username,
                password: self.webhook_basic_auth.password.into(),
            },
            report_basic_auth: DieselAdyenBasicAuth {
                username: self.report_basic_auth.username,
                password: self.report_basic_auth.password.into(),
            },
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        use diesel_models::ingestion_config::adyen::{
            AdyenBasicAuth as DieselAdyenBasicAuth,
            AdyenIngestionConfig as DieselAdyenIngestionConfig,
        };

        DieselAdyenIngestionConfig {
            hmac_secret: self.hmac_secret.into(),
            webhook_basic_auth: DieselAdyenBasicAuth {
                username: self.webhook_basic_auth.username,
                password: self.webhook_basic_auth.password.into(),
            },
            report_basic_auth: DieselAdyenBasicAuth {
                username: self.report_basic_auth.username,
                password: self.report_basic_auth.password.into(),
            },
        }
    }
}
