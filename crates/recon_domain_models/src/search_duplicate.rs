use crate::{
    behaviour::Conversion, encryption::ApplicationEncryptionInterface,
    type_encryption::CryptoOperation, EncryptionHandler,
};
use common_enums::SearchDuplicateStatus;
use common_types::field_path_key::FieldPathKey;
use common_utils::{
    crypto::EncryptableName,
    date_time,
    errors::{CustomResult, ValidationError},
    id_generation::generate_id_with_default_len,
    id_type::MerchantId,
    types::keymanager::Identifier,
};
use diesel_models::search_duplicate::{
    SearchDuplicate as DieselSearchDuplicate, SearchDuplicateNew,
};
use error_stack::ResultExt;
use masking::Secret;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainSearchDuplicate {
    pub search_duplicate_id: String,
    pub staging_entry_id: String,
    pub account_id: String,
    pub merchant_id: MerchantId,
    pub transformation_id: String,
    pub field_path: FieldPathKey,
    pub encrypted_field_value: EncryptableName,
    pub hashed_field_value: Secret<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub primary_staging_entry_id: String,
    pub staging_entry_version: i32,
    pub status: SearchDuplicateStatus,
}

#[derive(Clone, Debug)]
pub struct SearchDuplicateCreationData {
    pub staging_entry_id: String,
    pub account_id: String,
    pub merchant_id: MerchantId,
    pub transformation_id: String,
    pub field_path: FieldPathKey,
    pub encrypted_field_value: EncryptableName,
    pub hashed_field_value: Secret<String>,
    pub primary_staging_entry_id: String,
    pub staging_entry_version: i32,
}

impl DomainSearchDuplicate {
    pub fn new(data: SearchDuplicateCreationData) -> Self {
        let now = date_time::now();
        Self {
            search_duplicate_id: generate_id_with_default_len("sea_dup"),
            staging_entry_id: data.staging_entry_id,
            account_id: data.account_id,
            merchant_id: data.merchant_id,
            transformation_id: data.transformation_id,
            field_path: data.field_path,
            encrypted_field_value: data.encrypted_field_value,
            hashed_field_value: data.hashed_field_value,
            created_at: now,
            last_modified_at: now,
            primary_staging_entry_id: data.primary_staging_entry_id,
            staging_entry_version: data.staging_entry_version,
            status: SearchDuplicateStatus::Active,
        }
    }
}

#[async_trait::async_trait]
impl Conversion for DomainSearchDuplicate {
    type DstType = DieselSearchDuplicate;
    type NewDstType = SearchDuplicateNew;

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

        // Decrypt the field value using crypto_operation
        let encrypted_result = encryptor
            .client
            .crypto_operation(
                CryptoOperation::Decrypt(item.encrypted_field_value),
                identifier,
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt field value".to_string(),
            })?;

        let encrypted_field_value = encrypted_result.try_into_operation().change_context(
            ValidationError::InvalidValue {
                message: "Failed to convert decrypted field value".to_string(),
            },
        )?;

        Ok(Self {
            search_duplicate_id: item.id,
            staging_entry_id: item.logical_staging_entry_id,
            account_id: item.account_id,
            merchant_id,
            transformation_id: item.transformation_config_id,
            field_path: item.field_path,
            encrypted_field_value,
            hashed_field_value: item.hashed_field_value,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
            primary_staging_entry_id: item.primary_staging_entry_id,
            staging_entry_version: item.staging_entry_version,
            status: item.status,
        })
    }

    /// Convert domain search duplicate to diesel SearchDuplicateNew for insertion
    fn construct_new(self) -> Self::NewDstType {
        // Extract the already-encrypted field value
        let encrypted_field_value = self.encrypted_field_value.into();

        SearchDuplicateNew {
            id: self.search_duplicate_id,
            logical_staging_entry_id: self.staging_entry_id,
            account_id: self.account_id,
            merchant_id: self.merchant_id,
            transformation_config_id: self.transformation_id,
            field_path: self.field_path,
            encrypted_field_value,
            hashed_field_value: self.hashed_field_value,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            primary_staging_entry_id: self.primary_staging_entry_id,
            staging_entry_version: self.staging_entry_version,
            status: self.status,
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        DieselSearchDuplicate {
            id: self.search_duplicate_id,
            logical_staging_entry_id: self.staging_entry_id,
            account_id: self.account_id,
            merchant_id: self.merchant_id,
            transformation_config_id: self.transformation_id,
            field_path: self.field_path,
            encrypted_field_value: self.encrypted_field_value.into(),
            hashed_field_value: self.hashed_field_value,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            primary_staging_entry_id: self.primary_staging_entry_id,
            staging_entry_version: self.staging_entry_version,
            status: self.status,
        }
    }
}
