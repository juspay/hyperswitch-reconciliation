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
use diesel_models::business_profile::{
    BusinessProfile as DieselBusinessProfile, BusinessProfileNew,
};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainBusinessProfile {
    pub profile_id: String,
    pub profile_name: EncryptableName,
    pub merchant_id: MerchantId,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[async_trait::async_trait]
impl Conversion for DomainBusinessProfile {
    type DstType = DieselBusinessProfile;
    type NewDstType = BusinessProfileNew;

    /// Converts diesel model to domain model (decrypts sensitive fields from database)
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
            .crypto_operation(CryptoOperation::Decrypt(item.profile_name), identifier)
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Decryption failed".to_string(),
            })?;

        let profile_name = encrypted_result.try_into_operation().change_context(
            ValidationError::InvalidValue {
                message: "Failed to convert decrypted value".to_string(),
            },
        )?;

        Ok(Self {
            merchant_id,
            profile_id: item.profile_id,
            profile_name,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
        })
    }

    /// Convert domain business profile to diesel BusinessProfileNew for insertion
    fn construct_new(self) -> Self::NewDstType {
        // Extract the already-encrypted profile name
        let encrypted_profile_name = self.profile_name.into();

        BusinessProfileNew {
            profile_id: self.profile_id,
            profile_name: encrypted_profile_name,
            merchant_id: self.merchant_id,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        DieselBusinessProfile {
            profile_id: self.profile_id,
            profile_name: self.profile_name.into(),
            merchant_id: self.merchant_id,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }
}
