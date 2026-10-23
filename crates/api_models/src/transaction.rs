use crate::{
    entry::{CompactEntryData, EntryData, MetadataMismatchData},
    recon_rule::RuleData,
    user::User,
};
use common_enums::{BalanceDirection, Currency};
use common_types::{
    amount::AmountResponse,
    date_time::{TimeRange, TimeRangeValidation},
    field_path_key::FieldPathKey,
    pagination::{CursorPage, CursorPagination},
    transaction::{
        status::{TransactionAmountMismatchData, TransactionMatchedData, TransactionPostedData},
        TransactionListSortField,
    },
    units::FloatMajorUnit,
};
use common_utils::{crypto::SecretValue, custom_serde::query_parsers::parse_comma_separated};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use time::PrimitiveDateTime;
use utoipa::{IntoParams, ToSchema};

/// Search expectation data for expected entries
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SearchExpectation {
    #[schema(example = "rule_recon_001")]
    pub rule_id: String,
    #[schema(example = "payment_123")]
    pub source_value: String,
    #[schema(value_type = String, example = "metadata.transaction_id")]
    pub target_key: FieldPathKey,
}

/// Direct transaction entry - creates entry immediately
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DirectTransactionEntry {
    #[schema(example = "account_123")]
    pub account_id: String,
    pub entry_type: BalanceDirection,
    #[schema(example = 100.50)]
    pub amount: FloatMajorUnit,
    pub currency: Currency,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    #[schema(value_type = serde_json::Value)]
    pub metadata: SecretValue,
    #[schema(example = "stag_ent_abc123")]
    pub staging_entry_id: Option<String>,
    #[schema(example = "tran_conf_xyz789")]
    pub transformation_id: Option<String>,
    #[schema(example = "order_123456")]
    pub order_id: String,
}

/// Expected transaction entry - creates entry with search expectation
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExpectedTransactionEntry {
    #[schema(example = "account_456")]
    pub account_id: String,
    pub entry_type: BalanceDirection,
    #[schema(example = 100.50)]
    pub amount: FloatMajorUnit,
    pub currency: Currency,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    #[schema(value_type = serde_json::Value)]
    pub metadata: SecretValue,
    pub search_expectation: SearchExpectation,
    #[schema(example = "order_123456")]
    pub order_id: String,
}

/// Transaction entry that can be either direct or expected
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransactionEntry {
    Direct(DirectTransactionEntry),
    Expected(ExpectedTransactionEntry),
}

/// Request to create a new transaction with multiple entries
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TransactionRequest {
    pub entries: Vec<TransactionEntry>,
    #[schema(example = "rule_1234567890")]
    pub rule_id: String,
}

#[derive(Clone, Debug, Display, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", content = "sub_status", rename_all = "snake_case")]
pub enum ApiTransactionStatus {
    Expected,
    Posted(TransactionPostedData),
    OverAmount(TransactionAmountMismatchData),
    UnderAmount(TransactionAmountMismatchData),
    DataMismatch,
    CurrencyMismatch,
    SplitMismatch,
    Archived,
    Void,
    PartiallyReconciled,
    Missing,
    Matched(TransactionMatchedData),
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GetTransactionResponse {
    #[schema(example = "txn_1234567890")]
    pub id: String,
    #[schema(example = "ext_txn_abc123")]
    pub transaction_id: String,
    #[schema(example = json!(["entry_1", "entry_2"]))]
    pub entry_id: Vec<String>,
    #[serde(flatten)]
    pub status: ApiTransactionStatus,
    #[schema(example = 1)]
    pub version: i32,
    #[schema(example = "profile_1234567890")]
    pub profile_id: String,
    #[schema(example = "rule_1234567890")]
    pub rule_id: String,
    /// Credit amount with currency information
    pub credit_amount: AmountResponse,
    /// Debit amount with currency information
    pub debit_amount: AmountResponse,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<ApiTransactionStatus>,
    /// Original transaction data before it was discarded/archived (only present when transaction is archived)
    pub discarded_data: Option<TransactionData>,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    pub data: TransactionData,
    /// User who modified the transaction
    pub modified_by: Option<User>,
}

/// Transaction response with grouped entries containing flattened account data
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionWithEntriesResponse {
    #[schema(example = "txn_1234567890")]
    pub id: String,
    #[schema(example = "ext_txn_abc123")]
    pub transaction_id: String,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    #[serde(flatten)]
    pub status: ApiTransactionStatus,
    #[schema(example = 1)]
    pub version: i32,
    #[schema(example = "profile_1234567890")]
    pub profile_id: String,
    pub credit_amount: AmountResponse,
    pub debit_amount: AmountResponse,
    pub rule: RuleData,
    pub entries: Vec<CompactEntryData>,
    pub has_more_entries: bool,
    pub discarded_status: Option<ApiTransactionStatus>,
    pub data: TransactionData,
    pub discarded_data: Option<TransactionData>,
    pub discarded_at: Option<PrimitiveDateTime>,
    /// User who modified the transaction
    pub modified_by: Option<User>,
}

pub type TransactionListResponse = Vec<TransactionWithEntriesResponse>;

/// Simple data structure for API responses - flattened version of complex diesel model data
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TransactionData {
    Expected,
    Matched(MatchedData),
    Posted(PostedData),
    Mismatched,
    DataMismatch(MetadataMismatchData),
    Archived,
    Void { reason: Option<String> },
    PartiallyReconciled { reason: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "matched_data_type", rename_all = "snake_case")]
pub enum MatchedData {
    Force(ForceMatchedData),
    Manual(ManuallyMatchedData),
    Auto,
    WithTolerance,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct ForceMatchedData {
    #[schema(example = "Minor metadata differences acceptable for business purposes")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct ManuallyMatchedData {
    #[schema(example = "Minor metadata differences acceptable for business purposes")]
    pub reason: Option<String>,
}

/// Detailed posted data for API responses
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "posted_type", rename_all = "snake_case")]
pub enum PostedData {
    Manual(ManuallyPostedData),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct ManuallyPostedData {
    #[schema(example = "Transaction manually posted due to timing differences")]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VoidTransactionRequest {
    /// Reason for voiding the transaction
    #[schema(example = "Customer requested cancellation")]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionCreateEntriesRequest {
    /// New direct entries to add to resolve the transaction balance
    pub new_entries: Vec<DirectTransactionEntry>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForceReconcileRequest {
    /// Reason for force reconciliation
    #[schema(example = "Minor metadata differences acceptable for business purposes")]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EntryReplacement {
    /// ID of the entry to replace
    #[schema(example = "ent_1234567890")]
    pub entry_id: String,
    /// New entry data to replace the existing entry with
    pub entry_data: DirectTransactionEntry,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionReplaceEntriesRequest {
    /// Array of entries to replace with their new data
    pub entries: Vec<EntryReplacement>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReplaceEntryByStagingEntriesRequest {
    /// List of staging entry IDs to link to the transaction
    pub staging_entry_ids: Vec<String>,
    /// Specific entry ID to replace with staging entries
    #[schema(example = "entry_xyz789")]
    pub entry_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ManualReconciliationRequest {
    pub transaction_entries: Vec<TransactionEntryWithData>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionEntryWithData {
    #[schema(example = "account_123")]
    pub account_id: String,
    pub entry_type: BalanceDirection,
    #[schema(example = 100.50)]
    pub amount: FloatMajorUnit,
    pub currency: Currency,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    #[schema(value_type = serde_json::Value)]
    pub metadata: SecretValue,
    pub staging_entry_id: Option<String>,
    pub data: EntryData,
    #[schema(example = "order_123456")]
    pub order_id: String,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    Display,
    Deserialize,
    Serialize,
    EnumString,
    EnumIter,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ApiTransactionFilterStatus {
    #[default]
    Expected,
    PostedManual,
    OverAmountExpected,
    OverAmountMismatch,
    UnderAmountExpected,
    UnderAmountMismatch,
    DataMismatch,
    CurrencyMismatch,
    SplitMismatch,
    Archived,
    Void,
    PartiallyReconciled,
    Missing,
    MatchedAuto,
    MatchedManual,
    MatchedForce,
    MatchedWithTolerance,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct ApiTransactionAccountFilters {
    #[param(example = "txn_main_abc123")]
    pub transaction_id: Option<String>,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "posted_auto,expected", value_type = Vec<ApiTransactionFilterStatus>)]
    pub status: Option<Vec<ApiTransactionFilterStatus>>,

    #[serde(flatten, default)]
    #[param(value_type = TimeRange)]
    pub time_range: Option<TimeRange>,

    #[serde(default)]
    #[param(example = "rule_123")]
    pub rule_id: Option<String>,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "account_123,account_456", value_type = Vec<String>)]
    pub credit_account: Option<Vec<String>>,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "account_789,account_abc", value_type = Vec<String>)]
    pub debit_account: Option<Vec<String>>,
}

impl ApiTransactionAccountFilters {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(time_range) = &self.time_range {
            if let Some(end_time) = time_range.end_time {
                if time_range.start_time > end_time {
                    return Err("start_time cannot be greater than end_time".to_string());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct ApiTransactionListFiltersV2 {
    #[serde(default)]
    pub transaction_id: Option<String>,
    #[serde(default)]
    pub status: Option<Vec<ApiTransactionFilterStatus>>,
    #[serde(default)]
    pub time_range: Option<TimeRange>,
    #[serde(default)]
    pub rule_id: Option<String>,
    #[serde(default)]
    pub order_id: Option<String>,
    #[serde(default)]
    pub metadata: Vec<ApiTransactionListMetadataFilter>,
}

#[derive(Debug, Clone, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct ApiTransactionListMetadataFilter {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionListFilterValidationError {
    InvalidTimeRange,
    UnsupportedStatus { status: ApiTransactionFilterStatus },
}

impl ApiTransactionListFiltersV2 {
    pub fn validate(&self) -> Result<(), TransactionListFilterValidationError> {
        if let Some(time_range) = &self.time_range {
            time_range
                .validate()
                .map_err(|_| TransactionListFilterValidationError::InvalidTimeRange)?;
        }

        if let Some(statuses) = &self.status {
            if let Some(status) = statuses
                .iter()
                .find(|status| matches!(status, ApiTransactionFilterStatus::Archived))
            {
                return Err(TransactionListFilterValidationError::UnsupportedStatus {
                    status: *status,
                });
            }
        }

        Ok(())
    }

    pub fn contains_exactly_one_of_missing_or_expected(&self) -> bool {
        self.status.as_ref().is_some_and(|statuses| {
            let has_expected = statuses
                .iter()
                .any(|status| matches!(status, ApiTransactionFilterStatus::Expected));
            let has_missing = statuses
                .iter()
                .any(|status| matches!(status, ApiTransactionFilterStatus::Missing));
            has_expected != has_missing
        })
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ListTransactionsV2Request {
    #[serde(default)]
    pub filters: ApiTransactionListFiltersV2,
    pub cursor_payload: CursorPagination<TransactionListSortField>,
}

pub type TransactionListV2Page =
    CursorPage<TransactionWithEntriesResponse, TransactionListSortField>;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ManualReconciliationV2Request {
    pub entry_ops: Vec<EntryOp>,
    #[schema(example = "Linked the missing settlement line")]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EntryOp {
    UpdateEntry {
        #[schema(example = "entry_1234567890")]
        entry_id: String,
        #[serde(default)]
        overrides: EntryOverrides,
    },
    MarkReceived {
        #[schema(example = "entry_1234567890")]
        entry_id: String,
        #[serde(default)]
        overrides: EntryOverrides,
    },
    CreateEntry {
        entry: NewEntry,
    },
    CreateWithStagingEntry {
        #[schema(example = "stag_ent_1234567890")]
        staging_entry_id: String,
        #[serde(default)]
        overrides: StagingEntryOverrides,
    },
    ReplaceWithStagingEntry {
        #[schema(example = "entry_1234567890")]
        entry_id: String,
        #[schema(example = "stag_ent_1234567890")]
        staging_entry_id: String,
        #[serde(default)]
        overrides: StagingEntryOverrides,
    },
    RemoveEntry {
        #[schema(example = "entry_1234567890")]
        entry_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NewEntry {
    Direct {
        #[schema(example = "account_123")]
        account_id: String,
        entry_type: BalanceDirection,
        #[schema(example = 100.50)]
        amount: FloatMajorUnit,
        #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
        #[serde(with = "common_utils::custom_serde::iso8601")]
        effective_at: PrimitiveDateTime,
        #[schema(value_type = serde_json::Value)]
        metadata: SecretValue,
        #[schema(example = "order_123456")]
        order_id: String,
        #[schema(example = "tran_conf_xyz789")]
        transformation_id: Option<String>,
    },
    Expected {
        #[schema(example = "account_123")]
        account_id: String,
        entry_type: BalanceDirection,
        #[schema(example = 100.50)]
        amount: FloatMajorUnit,
        #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
        #[serde(with = "common_utils::custom_serde::iso8601")]
        effective_at: PrimitiveDateTime,
        #[schema(value_type = serde_json::Value)]
        metadata: SecretValue,
        #[schema(example = "order_123456")]
        order_id: String,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct EntryOverrides {
    pub entry_type: Option<BalanceDirection>,
    #[schema(example = 100.50)]
    pub amount: Option<FloatMajorUnit>,
    #[schema(value_type = Option<String>, example = "2024-01-01T00:00:00Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub effective_at: Option<PrimitiveDateTime>,
    #[schema(value_type = Option<serde_json::Value>)]
    pub metadata: Option<SecretValue>,
    #[schema(example = "order_123456")]
    pub order_id: Option<String>,
    #[schema(example = "transformation_123")]
    pub transformation_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct StagingEntryOverrides {
    #[schema(value_type = Option<String>, example = "2024-01-01T00:00:00Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub effective_at: Option<PrimitiveDateTime>,
    #[schema(value_type = Option<serde_json::Value>)]
    pub metadata: Option<SecretValue>,
    #[schema(example = "order_123456")]
    pub order_id: Option<String>,
}
