use crate::{
    behaviour::Conversion, encryption::ApplicationEncryptionInterface, entry::DomainEntry,
    transaction::SearchExpectationCreation, type_encryption::CryptoOperation, EncryptionHandler,
};
use api_models::search_expectation::SearchExpectationResponse;
use common_enums::ExpectationStatus;
use common_types::field_path_key::FieldPathKey;
use common_utils::{
    crypto::EncryptableName,
    errors::{CustomResult, ValidationError},
    id_generation::generate_id_with_default_len,
    id_type::MerchantId,
    types::keymanager::Identifier,
};
use diesel_models::search_expectation::{
    SearchExpectation as DieselSearchExpectation, SearchExpectationNew,
};
use error_stack::ResultExt;
use masking::Secret;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainSearchExpectation {
    pub search_expectation_id: String,
    pub primary_transaction_id: String,
    pub rule_id: String,
    pub target_account_id: String,
    pub target_key: FieldPathKey,
    pub status: ExpectationStatus,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub encrypted_source_value: EncryptableName,
    pub hashed_source_value: Secret<String>,
    pub merchant_id: MerchantId,
}

#[async_trait::async_trait]
impl Conversion for DomainSearchExpectation {
    type DstType = DieselSearchExpectation;
    type NewDstType = SearchExpectationNew;

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

        // Decrypt the source value using crypto_operation
        let encrypted_result = encryptor
            .client
            .crypto_operation(
                CryptoOperation::Decrypt(item.encrypted_source_value),
                identifier,
            )
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt source value".to_string(),
            })?;

        let encrypted_source_value = encrypted_result.try_into_operation().change_context(
            ValidationError::InvalidValue {
                message: "Failed to convert decrypted source value".to_string(),
            },
        )?;

        Ok(Self {
            search_expectation_id: item.id,
            primary_transaction_id: item.primary_transaction_id,
            rule_id: item.rule_id,
            target_account_id: item.target_account_id,
            target_key: item.target_key,
            status: item.status,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
            encrypted_source_value,
            hashed_source_value: item.hashed_source_value,
            merchant_id,
        })
    }

    /// Convert domain search expectation to diesel SearchExpectationNew for insertion
    fn construct_new(self) -> Self::NewDstType {
        // Extract the already-encrypted source value
        let encrypted_source_value = self.encrypted_source_value.into();

        SearchExpectationNew {
            id: self.search_expectation_id,
            primary_transaction_id: self.primary_transaction_id,
            rule_id: self.rule_id,
            target_account_id: self.target_account_id,
            target_key: self.target_key,
            status: self.status,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            encrypted_source_value,
            hashed_source_value: self.hashed_source_value,
            merchant_id: self.merchant_id,
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        DieselSearchExpectation {
            id: self.search_expectation_id,
            primary_transaction_id: self.primary_transaction_id,
            rule_id: self.rule_id,
            target_account_id: self.target_account_id,
            target_key: self.target_key,
            status: self.status,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            encrypted_source_value: self.encrypted_source_value.into(),
            hashed_source_value: self.hashed_source_value,
            merchant_id: self.merchant_id,
        }
    }
}

impl DomainSearchExpectation {
    /// Create a search expectation from creation data
    pub fn from_creation(
        data: SearchExpectationCreation,
        now: PrimitiveDateTime,
        primary_transaction_id: String,
    ) -> Self {
        Self {
            search_expectation_id: generate_id_with_default_len("sea_exp"),
            primary_transaction_id,
            rule_id: data.rule_id,
            target_account_id: data.target_account_id,
            target_key: data.target_key,
            status: ExpectationStatus::Expected,
            created_at: now,
            last_modified_at: now,
            encrypted_source_value: data.source_value,
            hashed_source_value: data.hashed_source_value,
            merchant_id: data.merchant_id,
        }
    }

    pub fn find_expected_entry(
        &self,
        mut entries: Vec<DomainEntry>,
    ) -> Option<(DomainEntry, Vec<DomainEntry>)> {
        entries
            .iter()
            .position(|entry| {
                entry.account_id == self.target_account_id
                    && entry.status == common_enums::EntryStatus::Expected
            })
            .map(|i| (entries.remove(i), entries))
    }

    pub fn to_api_response(self) -> SearchExpectationResponse {
        SearchExpectationResponse {
            id: self.search_expectation_id,
            primary_transaction_id: self.primary_transaction_id,
            rule_id: self.rule_id,
            target_account_id: self.target_account_id,
            target_key: self.target_key,
            status: self.status,
            encrypted_source_value: self.encrypted_source_value,
            hashed_source_value: self.hashed_source_value,
            merchant_id: self.merchant_id,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }
}
