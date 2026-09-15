use common_enums::{BalanceDirection, Currency, EntryStatus};
use common_types::{
    composite_field::{build_composite, CompositeDelimiter},
    entry::{
        BalanceDirectionMismatchData, CurrencyMismatchData, EntryAmountMismatchData, EntryData,
        EntryDataV2, EntryMismatchedData, EntryMismatchedDataV1, FieldMismatchDetail,
        MetadataMismatchData,
    },
    field_path_key::FieldPathKey,
    metadata_schema::ExposedKeysProvider,
    recon_rule::entry_field::{EntryField, ExtractAccountId, ExtractFromEntryField},
    transaction::status::DomainTransactionStatus,
    units::MinorUnit,
};
use common_utils::{crypto::EncryptableValue, id_type::MerchantId};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Secret};
use recon_domain_models::{
    behaviour::ReverseConversion,
    entry::{DomainEntry, EntryWithSearchableMetadata},
    search_expectation::DomainSearchExpectation,
    staging_entry::DomainStagingEntry,
    transaction::{
        DirectTransactionCreationEntry, DomainTransaction, ExpectedTransactionCreationEntry,
        SearchExpectationCreation, TransactionCreationEntry,
    },
};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use time::PrimitiveDateTime;

use crate::{
    db::{
        metadata_schema::MetadataSchemaInterface, search_expectation::SearchExpectationInterface,
        transformation::TransformationConfigInterface,
    },
    domain::recon_engine::{ProcessingError, ProcessingResult},
    routes::AppState,
};

pub const MAX_ENTRIES_PER_TRANSACTION: usize = 10_000;

/// Hash string for privacy-preserving duplicate detection
pub fn hash_string(value: &str) -> Secret<String> {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hash_bytes = hasher.finalize();
    let hash_hex = hex::encode(hash_bytes);

    Secret::new(hash_hex)
}

pub fn extract_unique_field(
    values: Vec<(EntryField, Option<String>)>,
    separator: CompositeDelimiter,
) -> ProcessingResult<(FieldPathKey, String)> {
    let separator = separator.to_string();

    let field_name = FieldPathKey::new(build_composite(
        values.iter().map(|(field, _)| field.to_string()),
        &separator,
    ));

    let field_value = build_composite(
        values
            .into_iter()
            .map(|(field, value)| {
                value.ok_or_else(|| report!(ProcessingError::MissingUniqueField(field)))
            })
            .collect::<Result<Vec<_>, _>>()?,
        &separator,
    );

    Ok((field_name, field_value))
}

pub async fn resolve_searchable_keys_by_config(
    state: &AppState,
    config_ids: HashSet<String>,
    profile_id: &str,
) -> ProcessingResult<HashMap<String, Vec<String>>> {
    // TODO: Move the fetching of transformation_configs and metadata_schema to prep
    let transformation_configs = state
        .store
        .list_transformation_configs_by_ids_and_profile_id(
            config_ids.into_iter().collect(),
            profile_id,
        )
        .await
        .change_context(ProcessingError::DatabaseError)?;

    let schemas = state
        .store
        .list_metadata_schemas_by_ids_and_profile_id(
            transformation_configs
                .iter()
                .map(|config| config.metadata_schema_id.clone())
                .collect(),
            profile_id,
        )
        .await
        .change_context(ProcessingError::DatabaseError)?;

    let searchable_keys_by_schema: HashMap<String, Vec<String>> = schemas
        .into_iter()
        .map(|schema| (schema.id, schema.schema_data.exposed_keys().searchable))
        .collect();

    transformation_configs
        .into_iter()
        .try_fold(HashMap::new(), |mut acc, config| {
            let config_id = config.id;
            let metadata_schema_id = config.metadata_schema_id;
            let searchable_keys = searchable_keys_by_schema
                .get(&metadata_schema_id)
                .cloned()
                .ok_or_else(|| {
                    report!(ProcessingError::InternalError(format!(
                        "metadata schema {metadata_schema_id} not found for transformation config {config_id}"
                    )))
                })?;
            acc.insert(config_id, searchable_keys);
            ProcessingResult::Ok(acc)
        })
}

pub fn searchable_keys_for_entry<'a>(
    entry: &TransactionCreationEntry,
    searchable_keys_by_config: &'a HashMap<String, Vec<String>>,
) -> ProcessingResult<&'a [String]> {
    match entry.transformation_id() {
        Some(config_id) => searchable_keys_by_config
            .get(config_id)
            .map(Vec::as_slice)
            .ok_or_else(|| {
                report!(ProcessingError::InternalError(format!(
                    "searchable keys not resolved for transformation config {config_id}"
                )))
            }),
        None => Ok(&[]),
    }
}

pub async fn couple_entries_with_searchable_metadata(
    state: &AppState,
    entries: Vec<TransactionCreationEntry>,
    profile_id: &str,
) -> ProcessingResult<Vec<EntryWithSearchableMetadata>> {
    let config_ids: HashSet<String> = entries
        .iter()
        .filter_map(|entry| entry.transformation_id().map(str::to_string))
        .collect();

    let searchable_keys_by_config =
        resolve_searchable_keys_by_config(state, config_ids, profile_id).await?;

    entries
        .into_iter()
        .map(|entry| {
            let searchable_keys = searchable_keys_for_entry(&entry, &searchable_keys_by_config)?;
            ProcessingResult::Ok(entry.into_entry_with_searchable_metadata(searchable_keys))
        })
        .collect()
}

pub trait DomainTransactionExt {
    /// Fetch the search expectation linked to this transaction for a given target account.
    /// Errors if zero or multiple expectations match.
    async fn fetch_search_expectation_from_db(
        &self,
        state: &AppState,
        target_account_id: &str,
        merchant_id: &MerchantId,
    ) -> ProcessingResult<DomainSearchExpectation>;
}

impl DomainTransactionExt for DomainTransaction {
    async fn fetch_search_expectation_from_db(
        &self,
        state: &AppState,
        target_account_id: &str,
        merchant_id: &MerchantId,
    ) -> ProcessingResult<DomainSearchExpectation> {
        let search_expectations = state
            .store
            .find_expected_expectations_by_primary_transaction_id_and_target_account_id(
                self.id.as_str(),
                target_account_id,
            )
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let search_expectation = match search_expectations.as_slice() {
            [search_expectation] => search_expectation.to_owned(),
            [] => return Err(ProcessingError::NoExpectationEntryFound.into()),
            _ => {
                return Err(ProcessingError::MultipleExpectedEntriesFound {
                    transaction_ids: search_expectations
                        .into_iter()
                        .map(|se| se.primary_transaction_id)
                        .collect(),
                }
                .into());
            }
        };

        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(merchant_id.clone());

        search_expectation
            .convert_back(&encryptor)
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to decrypt search expectation".to_string(),
            ))
    }
}

pub trait DomainEntryExt {
    async fn get_creation_data_using_db(
        self,
        state: &AppState,
        old_transaction: &DomainTransaction,
        new_transaction_status: DomainTransactionStatus,
    ) -> ProcessingResult<(TransactionCreationEntry, Option<SearchExpectationCreation>)>;
}

impl DomainEntryExt for DomainEntry {
    async fn get_creation_data_using_db(
        self,
        state: &AppState,
        old_transaction: &DomainTransaction,
        new_transaction_status: DomainTransactionStatus,
    ) -> ProcessingResult<(TransactionCreationEntry, Option<SearchExpectationCreation>)> {
        let new_data = match new_transaction_status {
            DomainTransactionStatus::Posted(_) => EntryData::V2(EntryDataV2::Posted),
            DomainTransactionStatus::Matched(_) => EntryData::V2(EntryDataV2::Matched),
            DomainTransactionStatus::Expected
            | DomainTransactionStatus::OverAmount(_)
            | DomainTransactionStatus::UnderAmount(_)
            | DomainTransactionStatus::DataMismatch
            | DomainTransactionStatus::CurrencyMismatch
            | DomainTransactionStatus::SplitMismatch
            | DomainTransactionStatus::Archived
            | DomainTransactionStatus::Void
            | DomainTransactionStatus::PartiallyReconciled => self.data.clone(),
        };

        match (&new_data).into() {
            EntryStatus::Expected => {
                let search_expectation = old_transaction
                    .fetch_search_expectation_from_db(state, &self.account_id, &self.merchant_id)
                    .await?;

                let creation_entry =
                    TransactionCreationEntry::Expected(ExpectedTransactionCreationEntry {
                        amount: self.amount,
                        currency: self.currency,
                        merchant_id: self.merchant_id.clone(),
                        data: new_data,
                        account_id: self.account_id,
                        entry_type: self.entry_type,
                        effective_at: self.effective_at,
                        metadata: self.metadata,
                        order_id: self.order_id,
                    });

                let expectation = SearchExpectationCreation {
                    rule_id: search_expectation.rule_id,
                    target_account_id: search_expectation.target_account_id,
                    source_value: search_expectation.encrypted_source_value,
                    hashed_source_value: search_expectation.hashed_source_value,
                    target_key: search_expectation.target_key,
                    merchant_id: self.merchant_id,
                };

                Ok((creation_entry, Some(expectation)))
            }
            EntryStatus::Void
            | EntryStatus::Posted
            | EntryStatus::Matched
            | EntryStatus::Pending
            | EntryStatus::Archived
            | EntryStatus::Mismatched => {
                let creation_entry =
                    TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
                        account_id: self.account_id,
                        entry_type: self.entry_type,
                        amount: self.amount,
                        currency: self.currency,
                        effective_at: self.effective_at,
                        merchant_id: self.merchant_id,
                        metadata: self.metadata,
                        data: new_data,
                        primary_staging_entry_id: self.staging_entry_id,
                        transformation_id: self.transformation_id,
                        order_id: self.order_id,
                        exposed_metadata: self.exposed_metadata,
                    });

                Ok((creation_entry, None))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BalanceState {
    Balanced,
    BalancedWithTolerance,
    UnderAmount,
    OverAmount,
}

#[derive(Clone, Debug)]
pub enum MatchOutcome {
    NoTarget,
    CurrencyMismatch,
    DataMismatch(BalanceState),
    Good(BalanceState),
}

#[derive(Clone, Debug)]
pub enum EntryClassification {
    Good,
    Expected,
    CurrencyMismatch(CurrencyMismatchData),
    MetadataMismatch(MetadataMismatchData),
    BalanceDirectionMismatch(BalanceDirectionMismatchData),
    AmountMismatch(EntryAmountMismatchData),
}

pub fn collect_metadata_mismatch_fields<'a>(
    classifications: impl IntoIterator<Item = &'a EntryClassification>,
) -> Vec<FieldMismatchDetail> {
    classifications
        .into_iter()
        .flat_map(|classification| match classification {
            EntryClassification::MetadataMismatch(MetadataMismatchData::V1(v1)) => {
                v1.mismatched_fields.clone()
            }
            _ => Vec::new(),
        })
        .collect()
}

#[derive(Clone)]
pub struct RawEntryData {
    pub account_id: String,
    pub amount: MinorUnit,
    pub entry_type: BalanceDirection,
    pub currency: Currency,
    pub effective_at: PrimitiveDateTime,
    pub order_id: String,
    pub metadata: EncryptableValue,
    pub merchant_id: MerchantId,
    pub primary_staging_entry_id: Option<String>,
    pub transformation_id: Option<String>,
    pub exposed_metadata: serde_json::Value,
}

impl RawEntryData {
    pub fn from_domain_entry(entry: DomainEntry) -> Self {
        Self {
            account_id: entry.account_id,
            amount: entry.amount,
            entry_type: entry.entry_type,
            currency: entry.currency,
            effective_at: entry.effective_at,
            order_id: entry.order_id,
            metadata: entry.metadata,
            merchant_id: entry.merchant_id,
            primary_staging_entry_id: entry.staging_entry_id,
            transformation_id: entry.transformation_id,
            exposed_metadata: entry.exposed_metadata,
        }
    }

    pub fn from_staging_entry(staging_entry: &DomainStagingEntry) -> Self {
        Self {
            account_id: staging_entry.account_id.clone(),
            amount: staging_entry.amount,
            entry_type: staging_entry.entry_type,
            currency: staging_entry.currency,
            effective_at: staging_entry.effective_at,
            order_id: staging_entry.order_id.clone(),
            metadata: staging_entry.metadata.clone(),
            merchant_id: staging_entry.merchant_id.clone(),
            primary_staging_entry_id: Some(staging_entry.id.clone()),
            transformation_id: Some(staging_entry.transformation_id.clone()),
            exposed_metadata: staging_entry.exposed_metadata.clone(),
        }
    }
}

pub struct EntryDataWithClassification {
    pub entry: RawEntryData,
    pub classification: EntryClassification,
}

impl EntryDataWithClassification {
    pub fn to_transaction_creation_entry(
        self,
        match_outcome: &MatchOutcome,
    ) -> TransactionCreationEntry {
        match self.classification {
            EntryClassification::Good => {
                TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
                    account_id: self.entry.account_id,
                    entry_type: self.entry.entry_type,
                    amount: self.entry.amount,
                    currency: self.entry.currency,
                    effective_at: self.entry.effective_at,
                    merchant_id: self.entry.merchant_id,
                    metadata: self.entry.metadata,
                    data: match match_outcome {
                        MatchOutcome::Good(BalanceState::Balanced)
                        | MatchOutcome::Good(BalanceState::BalancedWithTolerance) => {
                            EntryData::V2(EntryDataV2::Matched)
                        }
                        MatchOutcome::CurrencyMismatch
                        | MatchOutcome::DataMismatch(
                            BalanceState::Balanced
                            | BalanceState::BalancedWithTolerance
                            | BalanceState::UnderAmount
                            | BalanceState::OverAmount,
                        )
                        | MatchOutcome::NoTarget
                        | MatchOutcome::Good(
                            BalanceState::UnderAmount | BalanceState::OverAmount,
                        ) => EntryData::V2(EntryDataV2::Pending),
                    },
                    primary_staging_entry_id: self.entry.primary_staging_entry_id,
                    transformation_id: self.entry.transformation_id,
                    order_id: self.entry.order_id,
                    exposed_metadata: self.entry.exposed_metadata,
                })
            }
            EntryClassification::CurrencyMismatch(currency_mismatch_data) => {
                TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
                    account_id: self.entry.account_id,
                    entry_type: self.entry.entry_type,
                    amount: self.entry.amount,
                    currency: self.entry.currency,
                    effective_at: self.entry.effective_at,
                    merchant_id: self.entry.merchant_id,
                    metadata: self.entry.metadata,
                    data: EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                        EntryMismatchedDataV1::CurrencyMismatch(currency_mismatch_data),
                    ))),
                    primary_staging_entry_id: self.entry.primary_staging_entry_id,
                    transformation_id: self.entry.transformation_id,
                    order_id: self.entry.order_id,
                    exposed_metadata: self.entry.exposed_metadata,
                })
            }
            EntryClassification::MetadataMismatch(metadata_mismatch_data) => {
                TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
                    account_id: self.entry.account_id,
                    entry_type: self.entry.entry_type,
                    amount: self.entry.amount,
                    currency: self.entry.currency,
                    effective_at: self.entry.effective_at,
                    merchant_id: self.entry.merchant_id,
                    metadata: self.entry.metadata,
                    data: EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                        EntryMismatchedDataV1::MetadataMismatch(metadata_mismatch_data),
                    ))),
                    primary_staging_entry_id: self.entry.primary_staging_entry_id,
                    transformation_id: self.entry.transformation_id,
                    order_id: self.entry.order_id,
                    exposed_metadata: self.entry.exposed_metadata,
                })
            }
            EntryClassification::BalanceDirectionMismatch(balance_direction_mismatch_data) => {
                TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
                    account_id: self.entry.account_id,
                    entry_type: self.entry.entry_type,
                    amount: self.entry.amount,
                    currency: self.entry.currency,
                    effective_at: self.entry.effective_at,
                    merchant_id: self.entry.merchant_id,
                    metadata: self.entry.metadata,
                    data: EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                        EntryMismatchedDataV1::BalanceDirectionMismatch(
                            balance_direction_mismatch_data,
                        ),
                    ))),
                    primary_staging_entry_id: self.entry.primary_staging_entry_id,
                    transformation_id: self.entry.transformation_id,
                    order_id: self.entry.order_id,
                    exposed_metadata: self.entry.exposed_metadata,
                })
            }
            EntryClassification::AmountMismatch(amount_mismatch_data) => {
                TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
                    account_id: self.entry.account_id,
                    entry_type: self.entry.entry_type,
                    amount: self.entry.amount,
                    currency: self.entry.currency,
                    effective_at: self.entry.effective_at,
                    merchant_id: self.entry.merchant_id,
                    metadata: self.entry.metadata,
                    data: EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                        EntryMismatchedDataV1::AmountMismatch(amount_mismatch_data),
                    ))),
                    primary_staging_entry_id: self.entry.primary_staging_entry_id,
                    transformation_id: self.entry.transformation_id,
                    order_id: self.entry.order_id,
                    exposed_metadata: self.entry.exposed_metadata,
                })
            }
            EntryClassification::Expected => {
                TransactionCreationEntry::Expected(ExpectedTransactionCreationEntry {
                    account_id: self.entry.account_id,
                    entry_type: self.entry.entry_type,
                    amount: self.entry.amount,
                    currency: self.entry.currency,
                    effective_at: self.entry.effective_at,
                    merchant_id: self.entry.merchant_id,
                    metadata: self.entry.metadata,
                    data: EntryData::V2(EntryDataV2::Expected),
                    order_id: self.entry.order_id,
                })
            }
        }
    }
}

pub trait EntryDataWithClassificationVecExt {
    fn validate_transaction_entry_count(&self) -> ProcessingResult<()>;
}

impl EntryDataWithClassificationVecExt for Vec<EntryDataWithClassification> {
    fn validate_transaction_entry_count(&self) -> ProcessingResult<()> {
        let entry_count = self.len();

        if entry_count > MAX_ENTRIES_PER_TRANSACTION {
            return Err(report!(ProcessingError::InternalError(format!(
                "Transaction entry count {entry_count} exceeds limit {MAX_ENTRIES_PER_TRANSACTION}"
            ))));
        }

        Ok(())
    }
}

impl ExtractAccountId for RawEntryData {
    fn extract_account_id(&self) -> String {
        self.account_id.clone()
    }
}

impl ExtractFromEntryField for RawEntryData {
    fn amount(&self) -> MinorUnit {
        self.amount
    }

    fn currency(&self) -> Currency {
        self.currency
    }

    fn entry_type(&self) -> BalanceDirection {
        self.entry_type
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
}

pub fn extract_common_result<T, E, I>(
    iter: I,
    empty_err: impl FnOnce() -> E,
    mismatch_err: impl FnOnce() -> E,
) -> Result<T, E>
where
    I: IntoIterator<Item = Result<T, E>>,
    T: PartialEq,
{
    let mut iter = iter.into_iter();

    let first = iter.next().ok_or_else(empty_err)??;

    for item in iter {
        let item = item?;
        if item != first {
            return Err(mismatch_err());
        }
    }

    Ok(first)
}

pub fn extract_common<I, E>(
    iter: I,
    empty_err: impl FnOnce() -> E,
    mismatch_err: impl FnOnce() -> E,
) -> Result<I::Item, E>
where
    I: IntoIterator,
    I::Item: PartialEq,
{
    let mut iter = iter.into_iter();

    let first = match iter.next() {
        Some(item) => item,
        None => return Err(empty_err()),
    };

    for item in iter {
        if item != first {
            return Err(mismatch_err());
        }
    }

    Ok(first)
}
