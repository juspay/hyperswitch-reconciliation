use crate::{
    behaviour::Conversion, encryption::ApplicationEncryptionInterface,
    type_encryption::CryptoOperation, EncryptionHandler,
};
use common_utils::{
    crypto::EncryptableName,
    errors::{CustomResult, ValidationError},
    id_type::MerchantId,
    types::keymanager::Identifier,
};
use diesel_models::merchant_account::{
    MerchantAccount as DieselMerchantAccount, MerchantAccountNew,
};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainMerchantAccount {
    pub merchant_id: MerchantId,
    pub merchant_name: EncryptableName,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[async_trait::async_trait]
impl Conversion for DomainMerchantAccount {
    type DstType = DieselMerchantAccount;
    type NewDstType = MerchantAccountNew;

    /// Converts diesel model back to domain model (decrypts sensitive fields from database)
    async fn convert_back(
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let merchant_id = item.merchant_id.clone();
        let identifier = Identifier::ReconMerchant(merchant_id.clone());
        // Decrypt the merchant name directly using crypto_operation
        let encrypted_result = encryptor
            .client
            .crypto_operation(CryptoOperation::Decrypt(item.merchant_name), identifier)
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Decryption failed".to_string(),
            })?;

        let merchant_name = encrypted_result.try_into_operation().change_context(
            ValidationError::InvalidValue {
                message: "Failed to convert decrypted value".to_string(),
            },
        )?;

        Ok(Self {
            merchant_id,
            merchant_name,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
        })
    }

    /// Convert domain merchant account to diesel MerchantAccountNew for insertion
    fn construct_new(self) -> Self::NewDstType {
        // Extract the already-encrypted merchant name
        let encrypted_merchant_name = self.merchant_name.into();

        MerchantAccountNew {
            merchant_id: self.merchant_id,
            merchant_name: encrypted_merchant_name,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        DieselMerchantAccount {
            merchant_id: self.merchant_id,
            merchant_name: self.merchant_name.into(),
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }
}
