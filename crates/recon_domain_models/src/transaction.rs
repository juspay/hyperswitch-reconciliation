use common_enums::{BalanceDirection, Currency};
use common_types::{
    entry::EntryData,
    field_path_key::FieldPathKey,
    recon_rule::entry_field::{EntryField, ExtractAccountId, ExtractFromEntryField},
    transaction::{status::DomainTransactionStatus, TransactionData},
    units::MinorUnit,
};
use common_utils::{
    crypto::{EncryptableName, EncryptableValue},
    date_time,
    errors::{CustomResult, ValidationError},
    id_generation::generate_id_with_default_len,
    id_type::MerchantId,
};
use diesel_models::transaction::{Transaction, TransactionNew};
use masking::{ExposeInterface, Secret};
use time::PrimitiveDateTime;

use crate::{
    behaviour::Conversion,
    encryption::ApplicationEncryptionInterface,
    entry::{DomainEntry, EntryWithSearchableMetadata},
    search_entry_key::{DomainSearchEntryKey, SearchEntryKeyCreation},
    search_expectation::DomainSearchExpectation,
    search_group::DomainSearchGroup,
    EncryptionHandler,
};

/// Domain model for Transaction
#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainTransaction {
    pub id: String,
    pub transaction_id: String,
    pub entry_id: Vec<String>,
    pub created_at: PrimitiveDateTime,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub last_modified_at: PrimitiveDateTime,
    pub version: i32,
    pub profile_id: String,
    pub rule_id: String,
    pub credit_amount: MinorUnit,
    pub debit_amount: MinorUnit,
    pub currency: Currency,
    pub data: TransactionData,
    pub discarded_data: Option<TransactionData>,
    pub effective_at: PrimitiveDateTime,
    pub status: DomainTransactionStatus,
    pub discarded_status: Option<DomainTransactionStatus>,
    pub modified_by: Option<String>,
}

// ========================================
// Transaction Creation Data Structures
// ========================================

/// Input data for creating a transaction with all related entities
#[derive(Clone, Debug)]
pub struct TransactionCreationData {
    pub entries: Vec<EntryWithSearchableMetadata>,
    pub profile_id: String,
    pub rule_id: String,
    pub currency: Currency,
    pub data: TransactionData,
    pub search_group: Option<SearchGroupCreation>,
    pub search_expectations: Vec<SearchExpectationCreation>,
    pub modified_by: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TransactionUpdateData {
    pub entries: Vec<EntryWithSearchableMetadata>,
    pub profile_id: String,
    pub rule_id: String,
    pub currency: Currency,
    pub data: TransactionData,
    pub transaction_id: String,
    pub old_transaction_version: i32,
    pub search_group: Option<SearchGroupCreation>,
    /// Routing search expectations for this transaction version.
    pub search_expectations: Vec<SearchExpectationCreation>,
}

/// Entry creation data - either Direct or Expected
#[derive(Clone, Debug)]
pub enum TransactionCreationEntry {
    Direct(DirectTransactionCreationEntry),
    Expected(ExpectedTransactionCreationEntry),
}

/// Direct entry (main entry from staging entry)
#[derive(Clone, Debug)]
pub struct DirectTransactionCreationEntry {
    pub account_id: String,
    pub entry_type: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub effective_at: PrimitiveDateTime,
    pub merchant_id: MerchantId,
    pub metadata: EncryptableValue,
    pub data: EntryData,
    pub primary_staging_entry_id: Option<String>,
    pub transformation_id: Option<String>,
    pub order_id: String,
    pub exposed_metadata: serde_json::Value,
}

/// Expected entry (target placeholder awaiting a future confirmation). The
/// search expectation is owned at the transaction level rather than on
/// this entry — see `TransactionCreationData::search_expectations`.
#[derive(Clone, Debug)]
pub struct ExpectedTransactionCreationEntry {
    pub account_id: String,
    pub entry_type: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub effective_at: PrimitiveDateTime,
    pub merchant_id: MerchantId,
    pub metadata: EncryptableValue,
    pub data: EntryData,
    pub order_id: String,
}

/// Search expectation creation data
#[derive(Clone, Debug)]
pub struct SearchExpectationCreation {
    pub rule_id: String,
    pub target_account_id: String,
    pub source_value: EncryptableName,
    pub hashed_source_value: Secret<String>,
    pub target_key: FieldPathKey,
    pub merchant_id: MerchantId,
}

#[derive(Clone, Debug)]
pub struct SearchGroupCreation {
    pub merchant_id: MerchantId,
    pub encrypted_grouping_value: EncryptableName,
    pub hashed_grouping_value: Secret<String>,
}

/// Output: Transaction with all generated entities (with IDs)
#[derive(Clone, Debug)]
pub struct TransactionWithEntriesNew {
    pub transaction: DomainTransaction,
    pub entries: Vec<DomainEntry>,
    pub search_entry_keys: Vec<DomainSearchEntryKey>,
    pub search_group: Option<DomainSearchGroup>,
    pub search_expectations: Vec<DomainSearchExpectation>,
}

impl ExtractFromEntryField for TransactionCreationEntry {
    fn amount(&self) -> MinorUnit {
        match self {
            Self::Direct(d) => d.amount,
            Self::Expected(e) => e.amount,
        }
    }

    fn currency(&self) -> Currency {
        match self {
            Self::Direct(d) => d.currency,
            Self::Expected(e) => e.currency,
        }
    }

    fn effective_at(&self) -> PrimitiveDateTime {
        match self {
            Self::Direct(d) => d.effective_at,
            Self::Expected(e) => e.effective_at,
        }
    }

    fn entry_type(&self) -> BalanceDirection {
        match self {
            Self::Direct(d) => d.entry_type,
            Self::Expected(e) => e.entry_type,
        }
    }

    fn order_id(&self) -> String {
        match self {
            Self::Direct(d) => d.order_id.clone(),
            Self::Expected(e) => e.order_id.clone(),
        }
    }

    fn metadata(&self) -> &EncryptableValue {
        match self {
            Self::Direct(d) => &d.metadata,
            Self::Expected(e) => &e.metadata,
        }
    }

    fn extract_field(&self, field_name: &EntryField) -> Option<String> {
        match field_name {
            EntryField::Amount => Some(self.amount().to_string()),
            EntryField::Currency => Some(self.currency().to_string()),
            EntryField::EntryType => Some(self.entry_type().to_string()),
            EntryField::EffectiveAt => Some(self.effective_at().to_string()),
            EntryField::OrderId => Some(self.order_id().clone()),
            EntryField::Metadata(key) => {
                let metadata_json = self.metadata().get_inner().clone().expose();
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
}

impl ExtractAccountId for TransactionCreationEntry {
    fn extract_account_id(&self) -> String {
        match self {
            Self::Direct(d) => d.account_id.clone(),
            Self::Expected(e) => e.account_id.clone(),
        }
    }
}

impl TransactionCreationEntry {
    pub fn transformation_id(&self) -> Option<&str> {
        match self {
            Self::Direct(direct) => direct.transformation_id.as_deref(),
            Self::Expected(_) => None,
        }
    }

    pub fn primary_staging_entry_id(&self) -> Option<&str> {
        match self {
            Self::Direct(direct) => direct.primary_staging_entry_id.as_deref(),
            Self::Expected(_) => None,
        }
    }

    pub fn into_entry_with_searchable_metadata(
        self,
        searchable_keys: &[String],
    ) -> EntryWithSearchableMetadata {
        let searchable_metadata = match &self {
            Self::Direct(direct) => SearchEntryKeyCreation::rows_from_exposed_metadata(
                &direct.exposed_metadata,
                searchable_keys,
            ),
            Self::Expected(_) => Vec::new(),
        };

        EntryWithSearchableMetadata::new(self, searchable_metadata)
    }

    /// Convert entry creation data to a domain entry. Search expectations are
    /// owned at the transaction level (see `TransactionCreationData::search_expectations`),
    /// not on the entry, so this never returns one.
    pub fn get_entry(
        self,
        transaction_id: &str,
        now: PrimitiveDateTime,
        transaction_version: i32,
    ) -> DomainEntry {
        match self {
            Self::Direct(direct) => {
                DomainEntry::from_direct_creation(direct, transaction_id, transaction_version, now)
            }
            Self::Expected(expected) => DomainEntry::from_expected_creation(
                &expected,
                transaction_id,
                transaction_version,
                now,
            ),
        }
    }
}

impl TransactionCreationData {
    /// Creates a transaction with entries and search expectations.
    /// Generates transaction_id internally and propagates to all entities.
    /// This is a pure function - no encryption, no database operations.
    pub fn get_new_transaction_data(self) -> TransactionWithEntriesNew {
        let transaction_id = generate_id_with_default_len("txn_main");
        let primary_transaction_id = generate_id_with_default_len("txn");
        let now = date_time::now();

        let (entries, search_entry_keys) = self.entries.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut entries, mut search_entry_keys), entry_with_keys| {
                let (entry, search_keys) = entry_with_keys.get_entry_and_search_entry_keys(
                    &primary_transaction_id,
                    &transaction_id,
                    1,
                    &self.profile_id,
                    now,
                );
                search_entry_keys.extend(search_keys);
                entries.push(entry);
                (entries, search_entry_keys)
            },
        );

        // Calculate amounts using fold
        let (credit_amount, debit_amount) = entries.iter().fold(
            (MinorUnit::new(0), MinorUnit::new(0)),
            |(credit, debit), entry| match entry.entry_type {
                BalanceDirection::Credit => (
                    MinorUnit::new(credit.get_amount_as_i64() + entry.amount.get_amount_as_i64()),
                    debit,
                ),
                BalanceDirection::Debit => (
                    credit,
                    MinorUnit::new(debit.get_amount_as_i64() + entry.amount.get_amount_as_i64()),
                ),
            },
        );

        let search_group = self.search_group.map(|search_group| {
            DomainSearchGroup::from_creation(
                &self.rule_id,
                &primary_transaction_id,
                search_group.encrypted_grouping_value,
                search_group.hashed_grouping_value,
                search_group.merchant_id,
                &self.profile_id,
            )
        });

        let search_expectations: Vec<DomainSearchExpectation> = self
            .search_expectations
            .into_iter()
            .map(|creation| {
                DomainSearchExpectation::from_creation(
                    creation,
                    now,
                    primary_transaction_id.clone(),
                )
            })
            .collect();

        // Find the minimum effective_at from all entries
        let effective_at = entries
            .iter()
            .map(|entry| entry.effective_at)
            .min()
            .unwrap_or(now);

        let transaction = DomainTransaction {
            id: primary_transaction_id,
            transaction_id,
            entry_id: entries.iter().map(|e| e.entry_id.clone()).collect(),
            created_at: now,
            discarded_at: None,
            last_modified_at: now,
            version: 1,
            profile_id: self.profile_id,
            rule_id: self.rule_id,
            credit_amount,
            debit_amount,
            currency: self.currency,
            status: (&self.data).into(),
            data: self.data,
            discarded_data: None,
            effective_at,
            discarded_status: None,
            modified_by: self.modified_by,
        };

        TransactionWithEntriesNew {
            transaction,
            entries,
            search_entry_keys,
            search_group,
            search_expectations,
        }
    }
}

impl TransactionUpdateData {
    /// Creates a transaction with entries and search expectations.
    /// Generates transaction_id internally and propagates to all entities.
    /// This is a pure function - no encryption, no database operations.
    pub fn get_new_transaction_data(self) -> TransactionWithEntriesNew {
        let now = date_time::now();
        let new_version = self.old_transaction_version + 1;
        let primary_transaction_id = generate_id_with_default_len("txn");

        let (entries, search_entry_keys) = self.entries.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut entries, mut search_entry_keys), entry_with_keys| {
                let (entry, search_keys) = entry_with_keys.get_entry_and_search_entry_keys(
                    &primary_transaction_id,
                    &self.transaction_id,
                    new_version,
                    &self.profile_id,
                    now,
                );
                search_entry_keys.extend(search_keys);
                entries.push(entry);
                (entries, search_entry_keys)
            },
        );

        // Calculate amounts using fold
        let (credit_amount, debit_amount) = entries.iter().fold(
            (MinorUnit::new(0), MinorUnit::new(0)),
            |(credit, debit), entry| match entry.entry_type {
                BalanceDirection::Credit => (
                    MinorUnit::new(credit.get_amount_as_i64() + entry.amount.get_amount_as_i64()),
                    debit,
                ),
                BalanceDirection::Debit => (
                    credit,
                    MinorUnit::new(debit.get_amount_as_i64() + entry.amount.get_amount_as_i64()),
                ),
            },
        );

        let search_group = self.search_group.map(|search_group| {
            DomainSearchGroup::from_creation(
                &self.rule_id,
                &primary_transaction_id,
                search_group.encrypted_grouping_value,
                search_group.hashed_grouping_value,
                search_group.merchant_id,
                &self.profile_id,
            )
        });

        let search_expectations: Vec<DomainSearchExpectation> = self
            .search_expectations
            .into_iter()
            .map(|creation| {
                DomainSearchExpectation::from_creation(
                    creation,
                    now,
                    primary_transaction_id.clone(),
                )
            })
            .collect();

        // Find the minimum effective_at from all entries
        let effective_at = entries
            .iter()
            .map(|entry| entry.effective_at)
            .min()
            .unwrap_or(now);

        let transaction = DomainTransaction {
            id: primary_transaction_id,
            transaction_id: self.transaction_id,
            entry_id: entries.iter().map(|e| e.entry_id.clone()).collect(),
            created_at: now,
            discarded_at: None,
            last_modified_at: now,
            version: new_version,
            profile_id: self.profile_id,
            rule_id: self.rule_id,
            credit_amount,
            debit_amount,
            currency: self.currency,
            status: (&self.data).into(),
            data: self.data,
            discarded_status: None,
            discarded_data: None,
            effective_at,
            modified_by: None,
        };

        TransactionWithEntriesNew {
            transaction,
            entries,
            search_entry_keys,
            search_group,
            search_expectations,
        }
    }
}

#[async_trait::async_trait]
impl Conversion for DomainTransaction {
    type DstType = Transaction;
    type NewDstType = TransactionNew;

    /// Convert diesel model back to domain model (not implemented yet)
    async fn convert_back(
        _encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: item.id,
            transaction_id: item.transaction_id,
            entry_id: item.entry_ids,
            created_at: item.created_at,
            discarded_at: item.discarded_at,
            last_modified_at: item.last_modified_at,
            discarded_status: item.discarded_data.as_ref().map(|d| d.into()),
            version: item.version,
            profile_id: item.profile_id,
            rule_id: item.rule_id,
            credit_amount: item.credit_amount,
            debit_amount: item.debit_amount,
            currency: item.currency,
            status: (&item.data).into(),
            data: item.data,
            discarded_data: item.discarded_data,
            effective_at: item.effective_at,
            modified_by: item.modified_by,
        })
    }

    /// Convert domain transaction to diesel TransactionNew for insertion
    fn construct_new(self) -> Self::NewDstType {
        TransactionNew {
            id: self.id,
            transaction_id: self.transaction_id,
            entry_ids: self.entry_id,
            created_at: self.created_at,
            discarded_at: self.discarded_at,
            last_modified_at: self.last_modified_at,
            discarded_status: self
                .discarded_data
                .as_ref()
                .map(|d| DomainTransactionStatus::from(d).into()),
            version: self.version,
            profile_id: self.profile_id,
            rule_id: self.rule_id,
            credit_amount: self.credit_amount,
            debit_amount: self.debit_amount,
            currency: self.currency,
            data: self.data,
            discarded_data: self.discarded_data,
            effective_at: self.effective_at,
            status: self.status.into(),
            modified_by: self.modified_by,
        }
    }

    /// Convert domain model to diesel model
    fn convert(self) -> Self::DstType {
        Transaction {
            id: self.id,
            transaction_id: self.transaction_id,
            entry_ids: self.entry_id,
            created_at: self.created_at,
            discarded_at: self.discarded_at,
            last_modified_at: self.last_modified_at,
            discarded_status: self
                .discarded_data
                .as_ref()
                .map(|d| DomainTransactionStatus::from(d).into()),
            version: self.version,
            profile_id: self.profile_id,
            rule_id: self.rule_id,
            credit_amount: self.credit_amount,
            debit_amount: self.debit_amount,
            currency: self.currency,
            data: self.data,
            discarded_data: self.discarded_data,
            effective_at: self.effective_at,
            status: self.status.into(),
            modified_by: self.modified_by,
        }
    }
}
