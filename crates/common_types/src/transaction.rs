use crate::{
    date_time::TimeRange,
    entry::MetadataMismatchDataV1,
    transaction::status::{
        DomainTransactionStatus, TransactionAmountMismatchData, TransactionMatchedData,
        TransactionPostedData,
    },
};
use common_enums::DieselTransactionStatus;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

pub mod status;

#[derive(Debug, Clone)]
pub struct TransactionAccountFilters {
    pub transaction_id: Option<String>,
    pub status: Option<Vec<DieselTransactionStatus>>,
    pub time_range: Option<TimeRange>,
    pub rule_id: Option<String>,
    pub credit_account: Option<Vec<String>>,
    pub debit_account: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct TransactionListFiltersV2 {
    pub transaction_id: Option<String>,
    pub status: Option<Vec<TransactionListStatusFilter>>,
    pub time_range: Option<TimeRange>,
    pub rule_id: Option<String>,
    pub entry_fields: TransactionListEntryFilters,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionListStatusFilter {
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

impl TransactionListStatusFilter {
    pub fn is_expected(&self) -> bool {
        matches!(self, Self::Expected)
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }

    pub fn to_diesel_status(&self) -> DieselTransactionStatus {
        match self {
            Self::Expected | Self::Missing => DieselTransactionStatus::Expected,
            Self::PostedManual => DieselTransactionStatus::PostedManual,
            Self::OverAmountExpected => DieselTransactionStatus::OverAmountExpected,
            Self::OverAmountMismatch => DieselTransactionStatus::OverAmountMismatch,
            Self::UnderAmountExpected => DieselTransactionStatus::UnderAmountExpected,
            Self::UnderAmountMismatch => DieselTransactionStatus::UnderAmountMismatch,
            Self::DataMismatch => DieselTransactionStatus::DataMismatch,
            Self::CurrencyMismatch => DieselTransactionStatus::CurrencyMismatch,
            Self::SplitMismatch => DieselTransactionStatus::SplitMismatch,
            Self::Archived => DieselTransactionStatus::Archived,
            Self::Void => DieselTransactionStatus::Void,
            Self::PartiallyReconciled => DieselTransactionStatus::PartiallyReconciled,
            Self::MatchedAuto => DieselTransactionStatus::MatchedAuto,
            Self::MatchedManual => DieselTransactionStatus::MatchedManual,
            Self::MatchedForce => DieselTransactionStatus::MatchedForce,
            Self::MatchedWithTolerance => DieselTransactionStatus::MatchedWithTolerance,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionListEntryFilters {
    pub order_id: Option<String>,
    pub metadata: Vec<TransactionListMetadataFilters>,
}

#[derive(Debug, Clone)]
pub struct TransactionListMetadataFilters {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "sort_field", rename_all = "snake_case")]
pub enum TransactionListSortField {
    Id {
        id: Option<String>,
    },
    EffectiveAt {
        cursor_value: Option<TransactionEffectiveAtSortField>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct TransactionEffectiveAtSortField {
    #[schema(value_type = String)]
    #[serde(with = "common_utils::custom_serde::iso8601_micros")]
    pub effective_at: PrimitiveDateTime,
    pub id: String,
}

/// Data stored in transactions.data field with context for different statuses
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "data_version", rename_all = "snake_case")]
pub enum TransactionData {
    V3(TransactionDataV3),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TransactionDataV3 {
    Expected,
    Matched(MatchedData),
    Posted(PostedData),
    OverAmount(AmountMismatchData),
    UnderAmount(AmountMismatchData),
    DataMismatch(MetadataMismatchDataV1),
    CurrencyMismatch,
    SplitMismatch,
    Archived,
    Void(VoidData),
    PartiallyReconciled(PartiallyReconciledData),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "matched_data_version", rename_all = "snake_case")]
pub enum MatchedData {
    V1(MatchedDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "matched_data_type", rename_all = "snake_case")]
pub enum MatchedDataV1 {
    Force(ForceMatchedData),
    Manual(ManuallyMatchedData),
    Auto,
    WithTolerance,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "force_matched_data_version", rename_all = "snake_case")]
pub enum ForceMatchedData {
    V1(ForceMatchedDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ForceMatchedDataV1 {
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "manually_matched_data_version", rename_all = "snake_case")]
pub enum ManuallyMatchedData {
    V1(ManuallyMatchedDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ManuallyMatchedDataV1 {
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "amount_mismatch_data_version", rename_all = "snake_case")]
pub enum AmountMismatchData {
    V1(AmountMismatchDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "amount_mismatch_data_type", rename_all = "snake_case")]
pub enum AmountMismatchDataV1 {
    Mismatch,
    Expected,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "manual_posted_data_version", rename_all = "snake_case")]
pub enum ManualPostedData {
    V1(ManualPostedDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ManualPostedDataV1 {
    pub reason: Option<String>,
}

/// V2 Posted data for transactions
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "posted_version", rename_all = "snake_case")]
pub enum PostedData {
    V1(PostedDataV1),
}

/// V1 posted data with specific posted types
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "posted_type", rename_all = "snake_case")]
pub enum PostedDataV1 {
    Manual(ManualPostedData),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "void_data_version", rename_all = "snake_case")]
pub enum VoidData {
    V1(VoidDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct VoidDataV1 {
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "partially_reconciled_data_version", rename_all = "snake_case")]
pub enum PartiallyReconciledData {
    V1(PartiallyReconciledDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PartiallyReconciledDataV1 {
    pub reason: Option<String>,
}

common_utils::impl_to_sql_from_sql_json!(TransactionData);

impl From<&TransactionDataV3> for DomainTransactionStatus {
    fn from(value: &TransactionDataV3) -> Self {
        match value {
            TransactionDataV3::Archived => Self::Archived,
            TransactionDataV3::Expected => Self::Expected,
            TransactionDataV3::OverAmount(AmountMismatchData::V1(
                AmountMismatchDataV1::Expected,
            )) => Self::OverAmount(TransactionAmountMismatchData::Expected),
            TransactionDataV3::OverAmount(AmountMismatchData::V1(
                AmountMismatchDataV1::Mismatch,
            )) => Self::OverAmount(TransactionAmountMismatchData::Mismatch),
            TransactionDataV3::UnderAmount(AmountMismatchData::V1(
                AmountMismatchDataV1::Expected,
            )) => Self::UnderAmount(TransactionAmountMismatchData::Expected),
            TransactionDataV3::UnderAmount(AmountMismatchData::V1(
                AmountMismatchDataV1::Mismatch,
            )) => Self::UnderAmount(TransactionAmountMismatchData::Mismatch),
            TransactionDataV3::DataMismatch(_) => Self::DataMismatch,
            TransactionDataV3::CurrencyMismatch => Self::CurrencyMismatch,
            TransactionDataV3::SplitMismatch => Self::SplitMismatch,
            TransactionDataV3::Posted(PostedData::V1(PostedDataV1::Manual(_))) => {
                Self::Posted(TransactionPostedData::Manual)
            }
            TransactionDataV3::Matched(MatchedData::V1(MatchedDataV1::Force(_))) => {
                Self::Matched(TransactionMatchedData::Force)
            }
            TransactionDataV3::Matched(MatchedData::V1(MatchedDataV1::Manual(_))) => {
                Self::Matched(TransactionMatchedData::Manual)
            }
            TransactionDataV3::Matched(MatchedData::V1(MatchedDataV1::Auto)) => {
                Self::Matched(TransactionMatchedData::Auto)
            }
            TransactionDataV3::Matched(MatchedData::V1(MatchedDataV1::WithTolerance)) => {
                Self::Matched(TransactionMatchedData::WithTolerance)
            }
            TransactionDataV3::Void(_) => Self::Void,
            TransactionDataV3::PartiallyReconciled(_) => Self::PartiallyReconciled,
        }
    }
}

impl From<&TransactionData> for DomainTransactionStatus {
    fn from(value: &TransactionData) -> Self {
        match value {
            TransactionData::V3(v3_data) => v3_data.into(),
        }
    }
}
