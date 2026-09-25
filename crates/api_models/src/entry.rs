use common_enums::{BalanceDirection, Currency, EntryStatus};
use common_types::{
    amount::AmountResponse,
    date_time::{TimeRange, TimeRangeValidation},
    entry::EntryListSortField,
    pagination::{CursorPage, CursorPagination},
    recon_rule::entry_field::EntryField,
};
use common_utils::crypto::EncryptableValue;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::account::AccountData;

/// Complete entry data with flattened account information
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CompactEntryData {
    #[schema(example = "entry_1234567890")]
    pub entry_id: String,
    pub entry_type: BalanceDirection,
    pub status: EntryStatus,
    pub amount: AmountResponse,
    pub account: AccountData,
    #[schema(example = "order_123456")]
    pub order_id: String,
    pub staging_entry_id: Option<String>,
    pub mismatch: Option<MismatchedEntryData>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetEntryResponse {
    #[schema(example = "entry_123")]
    pub entry_id: String,
    #[schema(example = "account_123")]
    pub account_id: String,
    #[schema(example = "transaction_123")]
    pub transaction_id: String,
    pub entry_type: BalanceDirection,
    pub amount: AmountResponse,
    pub status: EntryStatus,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    #[schema(value_type = serde_json::Value)]
    pub metadata: EncryptableValue,
    pub exposed_metadata: serde_json::Value,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub discarded_at: Option<PrimitiveDateTime>,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub discarded_status: Option<EntryStatus>,
    pub version: i32,
    pub data: EntryData,
    pub discarded_data: Option<EntryData>,
    #[schema(example = "stag_ent_abc123")]
    pub staging_entry_id: Option<String>,
    #[schema(example = "tran_conf_xyz789")]
    pub transformation_id: Option<String>,
    #[schema(example = "order_123456")]
    pub order_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EntryData {
    Pending,
    Expected,
    Posted,
    Matched,
    Mismatched {
        #[serde(flatten)]
        mismatch_type: MismatchedEntryData,
    },
    Archived,
    Void,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(
    tag = "mismatch_type",
    content = "mismatch_data",
    rename_all = "snake_case"
)]
pub enum MismatchedEntryData {
    AmountMismatch(AmountMismatchData),
    BalanceDirectionMismatch(BalanceDirectionMismatchData),
    CurrencyMismatch(CurrencyMismatchData),
    MetadataMismatch(MetadataMismatchData),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AmountMismatchData {
    #[schema(example = 100.00)]
    pub expected_amount: AmountResponse,
    #[schema(example = 95.00)]
    pub actual_amount: AmountResponse,
}

/// Balance direction mismatch data for API responses
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BalanceDirectionMismatchData {
    #[schema(example = "credit")]
    pub expected_direction: BalanceDirection,
    #[schema(example = "debit")]
    pub actual_direction: BalanceDirection,
}

/// Currency mismatch data for API responses
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CurrencyMismatchData {
    #[schema(example = "USD")]
    pub expected_currency: Currency,
    #[schema(example = "EUR")]
    pub actual_currency: Currency,
}

/// Metadata mismatch data for API responses
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct MetadataMismatchData {
    pub mismatched_fields: Vec<FieldMismatchDetail>,
}

/// Details of a specific field mismatch for API responses
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct FieldMismatchDetail {
    #[schema(example = "metadata.description")]
    pub field_name: EntryField,
    #[schema(example = "Order Description")]
    pub label: Option<String>,
    #[schema(example = "Payment for order ABC")]
    pub expected_value: String,
    #[schema(example = "Payment for order XYZ")]
    pub actual_value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct ApiEntryListFilters {
    pub primary_transaction_id: String,
    #[serde(default)]
    pub account_ids: Option<Vec<String>>,
    #[serde(default)]
    pub entry_type: Option<BalanceDirection>,
    #[serde(default)]
    pub currency: Option<Vec<Currency>>,
    #[serde(default)]
    pub status: Option<Vec<EntryStatus>>,
    #[serde(default)]
    pub time_range: Option<TimeRange>,
    #[serde(default)]
    pub staging_entry_ids: Option<Vec<String>>,
    #[serde(default)]
    pub transformation_config_ids: Option<Vec<String>>,
    #[serde(default)]
    pub order_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryListFilterValidationError {
    InvalidTimeRange,
}

impl ApiEntryListFilters {
    pub fn validate(&self) -> Result<(), EntryListFilterValidationError> {
        self.time_range
            .map(|time_range| time_range.validate())
            .transpose()
            .map_err(|_| EntryListFilterValidationError::InvalidTimeRange)?;

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ListEntriesRequest {
    pub filters: ApiEntryListFilters,
    pub cursor_payload: CursorPagination<EntryListSortField>,
}

pub type EntryListPage = CursorPage<GetEntryResponse, EntryListSortField>;
