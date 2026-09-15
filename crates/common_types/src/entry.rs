//! Entry-related types used across the application

use crate::{date_time::TimeRange, recon_rule::entry_field::EntryField, units::MinorUnit};
use common_enums::{BalanceDirection, Currency, EntryStatus};
use common_utils::custom_serde::query_parsers::parse_comma_separated;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::{IntoParams, ToSchema};

/// Query parameters for listing entries
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListEntriesFilters {
    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(value_type = Vec<EntryStatus>)]
    pub status: Option<Vec<EntryStatus>>,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(value_type = Vec<BalanceDirection>)]
    pub entry_type: Option<Vec<BalanceDirection>>,

    #[serde(flatten, default)]
    #[param(value_type = TimeRange)]
    pub time_range: Option<TimeRange>,
}

impl ListEntriesFilters {
    /// Validates the time range filter
    ///
    /// Rules:
    /// - If end_time is provided, start_time must also be provided
    /// - If both are provided, start_time must be <= end_time
    /// - Only start_time without end_time is valid
    /// - Neither start_time nor end_time is valid (no time filtering)
    pub fn validate(&self) -> Result<(), String> {
        if let Some(time_range) = &self.time_range {
            if let Some(end_time) = time_range.end_time {
                if time_range.start_time > end_time {
                    return Err("start_time must be less than or equal to end_time".to_string());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EntryListFilters {
    pub logical_transaction_id: String,
    pub version: i32,
    pub account_ids: Option<Vec<String>>,
    pub entry_type: Option<BalanceDirection>,
    pub currency: Option<Vec<Currency>>,
    pub status: Option<Vec<EntryStatus>>,
    pub time_range: Option<TimeRange>,
    pub staging_entry_ids: Option<Vec<String>>,
    pub transformation_config_ids: Option<Vec<String>>,
    pub order_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "sort_field", rename_all = "snake_case")]
pub enum EntryListSortField {
    Id {
        id: Option<String>,
    },
    EffectiveAt {
        cursor_value: Option<EntryEffectiveAtSortField>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct EntryEffectiveAtSortField {
    #[schema(value_type = String)]
    #[serde(with = "common_utils::custom_serde::iso8601_micros")]
    pub effective_at: PrimitiveDateTime,
    pub id: String,
}

/// Data stored in entry.data field with context for different statuses
#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow, PartialEq, Eq,
)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "data_version", rename_all = "snake_case")]
pub enum EntryData {
    V2(EntryDataV2),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EntryDataV2 {
    Pending,
    Expected,
    Posted,
    Mismatched(EntryMismatchedData),
    Archived,
    Void,
    Matched,
}

impl From<&EntryData> for EntryStatus {
    fn from(value: &EntryData) -> Self {
        match value {
            EntryData::V2(data_v2) => data_v2.into(),
        }
    }
}

impl From<&EntryDataV2> for EntryStatus {
    fn from(value: &EntryDataV2) -> Self {
        match value {
            EntryDataV2::Pending => Self::Pending,
            EntryDataV2::Expected => Self::Expected,
            EntryDataV2::Posted => Self::Posted,
            EntryDataV2::Mismatched(_) => Self::Mismatched,
            EntryDataV2::Archived => Self::Archived,
            EntryDataV2::Void => Self::Void,
            EntryDataV2::Matched => Self::Matched,
        }
    }
}

common_utils::impl_to_sql_from_sql_json!(EntryData);

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "mismatched_data_version", rename_all = "snake_case")]
pub enum EntryMismatchedData {
    V1(EntryMismatchedDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "mismatch_type", rename_all = "snake_case")]
pub enum EntryMismatchedDataV1 {
    AmountMismatch(EntryAmountMismatchData),
    BalanceDirectionMismatch(BalanceDirectionMismatchData),
    CurrencyMismatch(CurrencyMismatchData),
    MetadataMismatch(MetadataMismatchData),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "amount_mismatch_version", rename_all = "snake_case")]
pub enum EntryAmountMismatchData {
    V1(EntryAmountMismatchDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct EntryAmountMismatchDataV1 {
    #[schema(example = 10000)]
    pub expected_amount: MinorUnit,
    #[schema(example = 9500)]
    pub actual_amount: MinorUnit,
    #[schema(example = "USD")]
    pub currency: Currency,
    #[schema(example = 500)]
    pub variance: MinorUnit,
}

/// Data for balance direction mismatch scenarios
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "balance_direction_mismatch_version", rename_all = "snake_case")]
pub enum BalanceDirectionMismatchData {
    V1(BalanceDirectionMismatchDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct BalanceDirectionMismatchDataV1 {
    #[schema(example = "credit")]
    pub expected_direction: BalanceDirection,
    #[schema(example = "debit")]
    pub actual_direction: BalanceDirection,
}

/// Data for currency mismatch scenarios
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "currency_mismatch_version", rename_all = "snake_case")]
pub enum CurrencyMismatchData {
    V1(CurrencyMismatchDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct CurrencyMismatchDataV1 {
    #[schema(example = "USD")]
    pub expected_currency: Currency,
    #[schema(example = "EUR")]
    pub actual_currency: Currency,
}

/// Data for metadata field mismatch scenarios
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "metadata_mismatch_version", rename_all = "snake_case")]
pub enum MetadataMismatchData {
    V1(MetadataMismatchDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct MetadataMismatchDataV1 {
    #[serde(default)]
    pub mismatched_fields: Vec<FieldMismatchDetail>,
}

/// Details of a specific field mismatch with versioning
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "metadata_mismatch_detail_version", rename_all = "snake_case")]
pub enum FieldMismatchDetail {
    V1(FieldMismatchDetailV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct FieldMismatchDetailV1 {
    #[schema(example = "description")]
    pub field_name: EntryField,
    #[serde(default)]
    pub label: Option<String>,
    #[schema(example = "Payment for order ABC")]
    pub expected_value: String,
    #[schema(example = "Payment for order XYZ")]
    pub actual_value: String,
}
