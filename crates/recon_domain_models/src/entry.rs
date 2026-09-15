use crate::{
    behaviour::Conversion,
    encryption::ApplicationEncryptionInterface,
    search_entry_key::{DomainSearchEntryKey, SearchEntryKeyCreation},
    transaction::{
        DirectTransactionCreationEntry, ExpectedTransactionCreationEntry, TransactionCreationEntry,
    },
    type_encryption::CryptoOperation,
    EncryptionHandler,
};
use common_enums::{BalanceDirection, Currency, EntryStatus};
use common_types::{
    entry::EntryData,
    recon_rule::entry_field::{EntryField, ExtractAccountId, ExtractFromEntryField},
    units::MinorUnit,
};
use common_utils::{
    crypto::EncryptableValue,
    errors::{CustomResult, ValidationError},
    id_generation::generate_id_with_default_len,
    id_type::MerchantId,
    types::keymanager::Identifier,
};
use diesel_models::entry::{Entry as DieselEntry, EntryNew};
use error_stack::ResultExt;
use masking::ExposeInterface;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainEntry {
    pub entry_id: String,
    pub account_id: String,
    pub transaction_id: String,
    pub entry_type: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub status: EntryStatus,
    pub effective_at: PrimitiveDateTime,
    pub metadata: EncryptableValue,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<EntryStatus>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub version: i32,
    pub merchant_id: MerchantId,
    pub data: EntryData,
    pub discarded_data: Option<EntryData>,
    pub staging_entry_id: Option<String>,
    pub transformation_id: Option<String>,
    pub order_id: String,
    pub exposed_metadata: serde_json::Value,
}

#[async_trait::async_trait]
impl Conversion for DomainEntry {
    type DstType = DieselEntry;
    type NewDstType = EntryNew;

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

        // Decrypt the metadata
        let encrypted_result = encryptor
            .client
            .crypto_operation(CryptoOperation::Decrypt(item.metadata), identifier)
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Metadata decryption failed".to_string(),
            })?;

        let metadata = encrypted_result.try_into_operation().change_context(
            ValidationError::InvalidValue {
                message: "Failed to convert decrypted metadata value".to_string(),
            },
        )?;

        Ok(Self {
            entry_id: item.id,
            account_id: item.account_id,
            transaction_id: item.logical_transaction_id,
            entry_type: item.balance_direction,
            amount: item.amount,
            currency: item.currency,
            status: item.status,
            effective_at: item.effective_at,
            metadata,
            discarded_at: item.discarded_at,
            discarded_status: item.discarded_status,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
            version: item.transaction_version,
            merchant_id,
            data: item.data,
            discarded_data: item.discarded_data,
            staging_entry_id: item.primary_staging_entry_id,
            transformation_id: item.transformation_config_id,
            order_id: item.entry_identifier,
            exposed_metadata: item.exposed_metadata,
        })
    }

    /// Convert domain entry to diesel EntryNew for insertion
    /// Convert domain entry to diesel EntryNew for insertion
    fn construct_new(self) -> Self::NewDstType {
        // Extract the already-encrypted metadata
        let encrypted_metadata = self.metadata.into();

        EntryNew {
            id: self.entry_id,
            account_id: self.account_id,
            logical_transaction_id: self.transaction_id,
            balance_direction: self.entry_type,
            amount: self.amount,
            currency: self.currency,
            status: self.status,
            effective_at: self.effective_at,
            metadata: encrypted_metadata,
            discarded_at: self.discarded_at,
            discarded_status: self.discarded_status,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            transaction_version: self.version,
            merchant_id: self.merchant_id,
            data: self.data,
            discarded_data: self.discarded_data,
            primary_staging_entry_id: self.staging_entry_id,
            transformation_config_id: self.transformation_id,
            entry_identifier: self.order_id,
            exposed_metadata: self.exposed_metadata,
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        DieselEntry {
            id: self.entry_id,
            account_id: self.account_id,
            logical_transaction_id: self.transaction_id,
            balance_direction: self.entry_type,
            amount: self.amount,
            currency: self.currency,
            status: self.status,
            effective_at: self.effective_at,
            metadata: self.metadata.into(),
            discarded_at: self.discarded_at,
            discarded_status: self.discarded_status,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            transaction_version: self.version,
            merchant_id: self.merchant_id,
            data: self.data,
            discarded_data: self.discarded_data,
            primary_staging_entry_id: self.staging_entry_id,
            transformation_config_id: self.transformation_id,
            entry_identifier: self.order_id,
            exposed_metadata: self.exposed_metadata,
        }
    }
}

impl ExtractFromEntryField for DomainEntry {
    fn extract_field(&self, field_name: &EntryField) -> Option<String> {
        match field_name {
            EntryField::Amount => Some(self.amount.to_string()),
            EntryField::Currency => Some(self.currency.to_string()),
            EntryField::EntryType => Some(self.entry_type.to_string()),
            EntryField::EffectiveAt => Some(self.effective_at.to_string()),
            EntryField::OrderId => Some(self.order_id.clone()),
            EntryField::Metadata(key) => {
                let metadata_json = self.metadata.get_inner().clone().expose();
                let data = metadata_json.get(key)?;

                match data {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    serde_json::Value::Null
                    | serde_json::Value::Array(_)
                    | serde_json::Value::Object(_) => None,
                }
            }
        }
    }

    fn currency(&self) -> Currency {
        self.currency
    }

    fn entry_type(&self) -> BalanceDirection {
        self.entry_type
    }

    fn amount(&self) -> MinorUnit {
        self.amount
    }

    fn metadata(&self) -> &EncryptableValue {
        &self.metadata
    }

    fn effective_at(&self) -> PrimitiveDateTime {
        self.effective_at
    }

    fn order_id(&self) -> String {
        self.order_id.clone()
    }
}

impl ExtractAccountId for DomainEntry {
    fn extract_account_id(&self) -> String {
        self.account_id.clone()
    }
}

impl DomainEntry {
    /// Create a direct entry from creation data
    pub fn from_direct_creation(
        data: DirectTransactionCreationEntry,
        transaction_id: &str,
        transaction_version: i32,
        now: PrimitiveDateTime,
    ) -> Self {
        Self {
            entry_id: generate_id_with_default_len("entry"),
            account_id: data.account_id,
            transaction_id: transaction_id.to_string(),
            entry_type: data.entry_type,
            amount: data.amount,
            currency: data.currency,
            status: (&data.data).into(),
            effective_at: data.effective_at,
            metadata: data.metadata,
            discarded_at: None,
            discarded_status: None,
            created_at: now,
            last_modified_at: now,
            version: transaction_version,
            merchant_id: data.merchant_id,
            data: data.data,
            discarded_data: None,
            staging_entry_id: data.primary_staging_entry_id,
            transformation_id: data.transformation_id,
            order_id: data.order_id,
            exposed_metadata: data.exposed_metadata,
        }
    }

    /// Create an expected entry from creation data
    pub fn from_expected_creation(
        data: &ExpectedTransactionCreationEntry,
        transaction_id: &str,
        transaction_version: i32,
        now: PrimitiveDateTime,
    ) -> Self {
        Self {
            entry_id: generate_id_with_default_len("entry"),
            account_id: data.account_id.clone(),
            transaction_id: transaction_id.to_string(),
            entry_type: data.entry_type,
            amount: data.amount,
            currency: data.currency,
            status: (&data.data).into(),
            effective_at: data.effective_at,
            metadata: data.metadata.clone(),
            discarded_at: None,
            discarded_status: None,
            created_at: now,
            last_modified_at: now,
            version: transaction_version,
            merchant_id: data.merchant_id.clone(),
            data: data.data.clone(),
            discarded_data: None,
            staging_entry_id: None,
            transformation_id: None,
            order_id: data.order_id.clone(),
            exposed_metadata: serde_json::json!({}),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EntryWithSearchableMetadata {
    pub entry: TransactionCreationEntry,
    pub searchable_metadata: Vec<SearchEntryKeyCreation>,
}

impl EntryWithSearchableMetadata {
    pub fn get_entry_and_search_entry_keys(
        self,
        primary_transaction_id: &str,
        transaction_id: &str,
        transaction_version: i32,
        profile_id: &str,
        now: PrimitiveDateTime,
    ) -> (DomainEntry, Vec<DomainSearchEntryKey>) {
        let entry = self
            .entry
            .get_entry(transaction_id, now, transaction_version);

        let search_entry_keys = self
            .searchable_metadata
            .into_iter()
            .map(|key| {
                key.get_search_entry_key(
                    profile_id.to_string(),
                    entry.entry_id.clone(),
                    primary_transaction_id.to_string(),
                )
            })
            .collect();

        (entry, search_entry_keys)
    }
}

impl EntryWithSearchableMetadata {
    pub fn new(
        entry: TransactionCreationEntry,
        searchable_metadata: Vec<SearchEntryKeyCreation>,
    ) -> Self {
        Self {
            entry,
            searchable_metadata,
        }
    }
}
