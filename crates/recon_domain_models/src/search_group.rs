use common_enums::SearchGroupStatus;
use common_utils::{
    crypto::EncryptableName,
    date_time,
    errors::{CustomResult, ValidationError},
    id_generation::generate_id_with_default_len,
    id_type::MerchantId,
    types::keymanager::Identifier,
};
use diesel_models::search_group::{SearchGroup as DieselSearchGroup, SearchGroupNew};
use error_stack::ResultExt;
use masking::Secret;
use serde::Serialize;
use time::PrimitiveDateTime;

use crate::{
    behaviour::Conversion, type_encryption::CryptoOperation, ApplicationEncryptionInterface,
    EncryptionHandler,
};

#[derive(Clone, Debug, Serialize)]
pub struct DomainSearchGroup {
    pub id: String,
    pub rule_id: String,
    pub primary_transaction_id: String,
    pub encrypted_grouping_value: EncryptableName,
    pub hashed_grouping_value: Secret<String>,
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub status: SearchGroupStatus,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

impl DomainSearchGroup {
    pub(crate) fn from_creation(
        rule_id: &str,
        primary_transaction_id: &str,
        encrypted_grouping_value: EncryptableName,
        hashed_grouping_value: Secret<String>,
        merchant_id: MerchantId,
        profile_id: &str,
    ) -> Self {
        let now = date_time::now();

        Self {
            id: generate_id_with_default_len("sea_grp"),
            rule_id: rule_id.to_string(),
            primary_transaction_id: primary_transaction_id.to_string(),
            encrypted_grouping_value,
            hashed_grouping_value,
            merchant_id,
            profile_id: profile_id.to_string(),
            status: SearchGroupStatus::Active,
            created_at: now,
            last_modified_at: now,
        }
    }
}

#[async_trait::async_trait]
impl Conversion for DomainSearchGroup {
    type DstType = DieselSearchGroup;
    type NewDstType = SearchGroupNew;

    async fn convert_back(
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let merchant_id = item.merchant_id.clone();
        let identifier = Identifier::ReconMerchant(merchant_id.clone());

        let encrypted_result = encryptor
            .client
            .crypto_operation(
                CryptoOperation::Decrypt(item.encrypted_grouping_value),
                identifier,
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt grouping value".to_string(),
            })?;

        let encrypted_grouping_value = encrypted_result.try_into_operation().change_context(
            ValidationError::InvalidValue {
                message: "Failed to convert decrypted grouping value".to_string(),
            },
        )?;

        Ok(Self {
            id: item.id,
            rule_id: item.rule_id,
            primary_transaction_id: item.primary_transaction_id,
            encrypted_grouping_value,
            hashed_grouping_value: item.hashed_grouping_value,
            merchant_id: item.merchant_id,
            profile_id: item.profile_id,
            status: item.status,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
        })
    }

    fn construct_new(self) -> Self::NewDstType {
        let encrypted_grouping_value = self.encrypted_grouping_value.into();

        SearchGroupNew {
            id: self.id,
            rule_id: self.rule_id,
            primary_transaction_id: self.primary_transaction_id,
            encrypted_grouping_value,
            hashed_grouping_value: self.hashed_grouping_value,
            merchant_id: self.merchant_id,
            profile_id: self.profile_id,
            status: self.status,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }

    fn convert(self) -> Self::DstType {
        DieselSearchGroup {
            id: self.id,
            rule_id: self.rule_id,
            primary_transaction_id: self.primary_transaction_id,
            encrypted_grouping_value: self.encrypted_grouping_value.into(),
            hashed_grouping_value: self.hashed_grouping_value,
            merchant_id: self.merchant_id,
            profile_id: self.profile_id,
            status: self.status,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }
}
