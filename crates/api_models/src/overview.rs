use crate::staging_entry::{ApiStagingEntryCoarseStatus, ApiStagingEntryStatus};
use crate::transaction::ApiTransactionFilterStatus;
use common_enums::{BalanceDirection, Granularity, ReconRuleAccountType};
use common_types::{
    amount::AmountResponse,
    date_time::{TimeRange, TimeRangeValidation},
};
use common_utils::custom_serde::query_parsers::parse_comma_separated;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct OverviewFilters {
    #[serde(flatten)]
    #[param(value_type = TimeRange)]
    pub time_range: TimeRange,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "rule_123,rule_456", value_type = Vec<String>)]
    pub rule_ids: Option<Vec<String>>,
}

impl OverviewFilters {
    pub fn validate(&self) -> Result<(), String> {
        self.time_range.validate()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct TimeSeriesFilters {
    #[serde(flatten)]
    #[param(value_type = TimeRange)]
    pub time_range: TimeRange,
    pub granularity: Granularity,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "rule_123,rule_456", value_type = Vec<String>)]
    pub rule_ids: Option<Vec<String>>,
}

impl TimeSeriesFilters {
    pub fn validate(&self) -> Result<(), String> {
        self.time_range.validate()
    }
}

/// One `(status, currency)` aggregate for a transaction rule.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionStatusAmount {
    pub status: ApiTransactionFilterStatus,
    #[schema(example = 12)]
    pub count: i64,
    pub credit_amount: AmountResponse,
    pub debit_amount: AmountResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountTransactionStatusAmount {
    pub status: ApiTransactionFilterStatus,
    #[schema(example = 8)]
    pub credit_count: i64,
    #[schema(example = 4)]
    pub debit_count: i64,
    pub credit_amount: AmountResponse,
    pub debit_amount: AmountResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuleTransactionsOverview {
    #[schema(example = "rule_1234567890")]
    pub rule_id: String,
    #[schema(example = "Rule 1")]
    pub rule_name: String,
    pub status_breakdown: Vec<TransactionStatusAmount>,
}

pub type TransactionsOverviewResponse = Vec<RuleTransactionsOverview>;

#[derive(Debug, Clone, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct StagingEntryOverviewFilters {
    #[serde(flatten)]
    #[param(value_type = TimeRange)]
    pub time_range: TimeRange,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "account_123,account_456", value_type = Vec<String>)]
    pub account_ids: Option<Vec<String>>,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "trans_his_123,trans_his_456", value_type = Vec<String>)]
    pub transformation_history_ids: Option<Vec<String>>,
}

impl StagingEntryOverviewFilters {
    pub fn validate(&self) -> Result<(), String> {
        self.time_range.validate()
    }
}

/// One `(status, currency)` aggregate for a staging entry account.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StagingEntryStatusAmount {
    pub status: ApiStagingEntryCoarseStatus,
    #[schema(example = 12)]
    pub count: i64,
    pub credit_amount: AmountResponse,
    pub debit_amount: AmountResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StagingEntryDetailedStatusAmount {
    #[serde(flatten)]
    pub detailed_status: ApiStagingEntryStatus,
    #[schema(example = 12)]
    pub count: i64,
    pub credit_amount: AmountResponse,
    pub debit_amount: AmountResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountStagingEntriesOverview {
    #[schema(example = "account_1234567890")]
    pub account_id: String,
    #[schema(example = "PSP Settlement Account")]
    pub account_name: String,
    pub account_type: BalanceDirection,
    pub status_breakdown: Vec<StagingEntryStatusAmount>,
    pub detailed_status_breakdown: Vec<StagingEntryDetailedStatusAmount>,
}

pub type StagingEntriesOverviewResponse = Vec<AccountStagingEntriesOverview>;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionTimeBucket {
    /// Half-open `[start_time, end_time)` window this bucket covers, clipped to the requested range.
    pub time_range: TimeRange,
    pub status_breakdown: Vec<TransactionStatusAmount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuleTransactionsTimeSeries {
    #[schema(example = "rule_1234567890")]
    pub rule_id: String,
    #[schema(example = "Rule 1")]
    pub rule_name: String,
    pub time_series: Vec<TransactionTimeBucket>,
}

pub type TransactionsTimeSeriesResponse = Vec<RuleTransactionsTimeSeries>;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountStatusOverview {
    #[schema(example = "account_1234567890")]
    pub account_id: String,
    #[schema(example = "PSP Settlement Account")]
    pub account_name: String,
    pub account_type: BalanceDirection,
    pub rule_account_type: ReconRuleAccountType,
    pub status_breakdown: Vec<AccountTransactionStatusAmount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuleAccountsOverview {
    #[schema(example = "rule_1234567890")]
    pub rule_id: String,
    #[schema(example = "Rule 1")]
    pub rule_name: String,
    pub accounts: Vec<AccountStatusOverview>,
}

pub type RuleAccountsOverviewResponse = Vec<RuleAccountsOverview>;
