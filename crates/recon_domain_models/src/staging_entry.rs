use crate::{
    behaviour::Conversion, encryption::ApplicationEncryptionInterface,
    type_encryption::CryptoOperation, EncryptionHandler,
};
use common_enums::{BalanceDirection, Currency, StagingEntryProcessingMode};
use common_types::{
    recon_rule::entry_field::{EntryField, ExtractAccountId, ExtractFromEntryField},
    staging_entry::{status::DomainStagingEntryStatus, StagingEntryData},
    units::MinorUnit,
};
use common_utils::{
    crypto::EncryptableValue,
    date_time,
    errors::{CustomResult, ValidationError},
    id_generation::generate_id_with_default_len,
    id_type::MerchantId,
    types::keymanager::Identifier,
};
use diesel_models::staging_entry::{StagingEntry as DieselStagingEntry, StagingEntryNew};
use error_stack::ResultExt;
use masking::ExposeInterface;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainStagingEntry {
    pub id: String,
    pub staging_entry_id: String,
    pub account_id: String,
    pub merchant_id: MerchantId,
    pub entry_type: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub status: DomainStagingEntryStatus,
    pub effective_at: PrimitiveDateTime,
    pub metadata: EncryptableValue,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub processing_mode: StagingEntryProcessingMode,
    pub profile_id: String,
    pub transformation_id: String,
    pub transformation_history_id: Option<String>,
    pub data: StagingEntryData,
    pub version: i32,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<DomainStagingEntryStatus>,
    pub discarded_data: Option<StagingEntryData>,
    pub order_id: String,
    pub modified_by: Option<String>,
    pub exposed_metadata: serde_json::Value,
    pub searchable_metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum StagingEntryDomainVersionUpdate {
    ProcessedUpdate { data: StagingEntryData },
    NeedsManualReview { data: StagingEntryData },
    VoidUpdate { data: StagingEntryData },
}

#[async_trait::async_trait]
impl Conversion for DomainStagingEntry {
    type DstType = DieselStagingEntry;
    type NewDstType = StagingEntryNew;

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
            id: item.id,
            staging_entry_id: item.staging_entry_id,
            account_id: item.account_id,
            merchant_id,
            entry_type: item.balance_direction,
            amount: item.amount,
            currency: item.currency,
            status: item.status.into(),
            effective_at: item.effective_at,
            metadata,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
            processing_mode: item.processing_mode,
            profile_id: item.profile_id,
            transformation_id: item.transformation_config_id,
            transformation_history_id: item.transformation_history_id,
            data: item.data,
            version: item.version,
            discarded_at: item.discarded_at,
            discarded_status: item.discarded_status.map(Into::into),
            discarded_data: item.discarded_data,
            order_id: item.entry_identifier,
            modified_by: item.modified_by,
            exposed_metadata: item.exposed_metadata,
            searchable_metadata: item.searchable_metadata,
        })
    }

    /// Convert domain staging entry to diesel StagingEntryNew for insertion
    fn construct_new(self) -> Self::NewDstType {
        // Extract the already-encrypted metadata
        let encrypted_metadata = self.metadata.into();

        StagingEntryNew {
            id: self.id,
            staging_entry_id: self.staging_entry_id,
            account_id: self.account_id,
            balance_direction: self.entry_type,
            amount: self.amount,
            currency: self.currency,
            status: self.status.into(),
            effective_at: self.effective_at,
            metadata: encrypted_metadata,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            processing_mode: self.processing_mode,
            profile_id: self.profile_id,
            transformation_config_id: self.transformation_id,
            transformation_history_id: self.transformation_history_id,
            merchant_id: self.merchant_id,
            data: self.data,
            version: self.version,
            discarded_at: self.discarded_at,
            discarded_status: self.discarded_status.map(Into::into),
            discarded_data: self.discarded_data,
            entry_identifier: self.order_id,
            modified_by: self.modified_by,
            exposed_metadata: self.exposed_metadata,
            searchable_metadata: self.searchable_metadata,
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        DieselStagingEntry {
            id: self.id,
            staging_entry_id: self.staging_entry_id,
            account_id: self.account_id,
            balance_direction: self.entry_type,
            amount: self.amount,
            currency: self.currency,
            status: self.status.into(),
            effective_at: self.effective_at,
            metadata: self.metadata.into(),
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            processing_mode: self.processing_mode,
            profile_id: self.profile_id,
            transformation_config_id: self.transformation_id,
            transformation_history_id: self.transformation_history_id,
            merchant_id: self.merchant_id,
            data: self.data,
            version: self.version,
            discarded_at: self.discarded_at,
            discarded_status: self.discarded_status.map(Into::into),
            discarded_data: self.discarded_data,
            entry_identifier: self.order_id,
            modified_by: self.modified_by,
            exposed_metadata: self.exposed_metadata,
            searchable_metadata: self.searchable_metadata,
        }
    }
}

impl DomainStagingEntry {
    pub fn new_versioned(
        self,
        update: StagingEntryDomainVersionUpdate,
        modified_by: Option<String>,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("stag_ent"); // Primary key

        let data = match update {
            StagingEntryDomainVersionUpdate::ProcessedUpdate { data }
            | StagingEntryDomainVersionUpdate::NeedsManualReview { data }
            | StagingEntryDomainVersionUpdate::VoidUpdate { data } => data,
        };
        let status = DomainStagingEntryStatus::from(&data);

        Self {
            id,
            staging_entry_id: self.staging_entry_id,
            account_id: self.account_id,
            merchant_id: self.merchant_id,
            entry_type: self.entry_type,
            amount: self.amount,
            currency: self.currency,
            status,
            effective_at: self.effective_at,
            metadata: self.metadata,
            created_at: now,
            last_modified_at: now,
            processing_mode: self.processing_mode,
            profile_id: self.profile_id,
            transformation_id: self.transformation_id,
            transformation_history_id: self.transformation_history_id,
            data,
            version: self.version + 1,
            discarded_at: None,
            discarded_status: None,
            discarded_data: None,
            order_id: self.order_id,
            modified_by,
            exposed_metadata: self.exposed_metadata,
            searchable_metadata: self.searchable_metadata,
        }
    }

    /// Converts domain model to diesel model (converts EncryptableValue back to Encryption)
    pub fn convert(self) -> DieselStagingEntry {
        DieselStagingEntry {
            id: self.id,
            staging_entry_id: self.staging_entry_id,
            account_id: self.account_id,
            merchant_id: self.merchant_id,
            balance_direction: self.entry_type,
            amount: self.amount,
            currency: self.currency,
            status: self.status.into(),
            effective_at: self.effective_at,
            metadata: self.metadata.into(),
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
            processing_mode: self.processing_mode,
            profile_id: self.profile_id,
            transformation_config_id: self.transformation_id,
            transformation_history_id: self.transformation_history_id,
            data: self.data,
            version: self.version,
            discarded_at: self.discarded_at,
            discarded_status: self.discarded_status.map(Into::into),
            discarded_data: self.discarded_data,
            entry_identifier: self.order_id,
            modified_by: self.modified_by,
            exposed_metadata: self.exposed_metadata,
            searchable_metadata: self.searchable_metadata,
        }
    }
}

impl ExtractFromEntryField for DomainStagingEntry {
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

impl ExtractAccountId for DomainStagingEntry {
    fn extract_account_id(&self) -> String {
        self.account_id.clone()
    }
}
