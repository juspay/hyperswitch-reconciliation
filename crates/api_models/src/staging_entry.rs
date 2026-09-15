use crate::{account::AccountData, transformation::ApiTransformationConfigData, user::User};
use common_enums::{BalanceDirection, Currency, StagingEntryProcessingMode};
use common_types::{
    amount::AmountResponse,
    date_time::{TimeRange, TimeRangeValidation},
    pagination::{CursorPage, CursorPagination},
    recon_rule::entry_field::EntryField,
    staging_entry::{status::StagingEntryManualReviewData, StagingEntryListSortField},
    units::FloatMajorUnit,
};
use common_utils::{
    crypto::{EncryptableValue, SecretValue},
    custom_serde::query_parsers::parse_comma_separated,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumIter, EnumString};
use time::PrimitiveDateTime;
use utoipa::{IntoParams, ToSchema};

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Hash, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", content = "sub_status", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ApiStagingEntryStatus {
    Pending,
    // TODO: make the sub-status mandatory (drop `Option`) once the frontend always sends it. Until
    // then a bare `{"status": "needs_manual_review"}` with the `sub_status` key absent has to keep
    // matching every sub-status.
    NeedsManualReview(Option<StagingEntryManualReviewData>),
    Processed,
    Void,
    Archived,
}
// TODO: delete once the frontend has moved onto `detailed_status`.
#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    EnumString,
    EnumIter,
    PartialEq,
    Eq,
    Hash,
    Deserialize,
    Serialize,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ApiStagingEntryCoarseStatus {
    Pending,
    NeedsManualReview,
    Processed,
    Void,
    Archived,
}

impl ApiStagingEntryStatus {
    pub fn coarse(self) -> ApiStagingEntryCoarseStatus {
        match self {
            Self::Pending => ApiStagingEntryCoarseStatus::Pending,
            Self::NeedsManualReview(_) => ApiStagingEntryCoarseStatus::NeedsManualReview,
            Self::Processed => ApiStagingEntryCoarseStatus::Processed,
            Self::Void => ApiStagingEntryCoarseStatus::Void,
            Self::Archived => ApiStagingEntryCoarseStatus::Archived,
        }
    }
}

impl ApiStagingEntryCoarseStatus {
    pub fn to_detailed(self) -> ApiStagingEntryStatus {
        match self {
            Self::Pending => ApiStagingEntryStatus::Pending,
            Self::NeedsManualReview => ApiStagingEntryStatus::NeedsManualReview(None),
            Self::Processed => ApiStagingEntryStatus::Processed,
            Self::Void => ApiStagingEntryStatus::Void,
            Self::Archived => ApiStagingEntryStatus::Archived,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StagingEntryCreateRequest {
    #[schema(example = "account_123")]
    pub account_id: String,
    pub entry_type: BalanceDirection,
    #[schema(example = 99.99)]
    pub amount: FloatMajorUnit,
    pub currency: Currency,
    pub processing_mode: StagingEntryProcessingMode,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    #[schema(value_type = serde_json::Value)]
    pub metadata: SecretValue,
    #[schema(example = "tran_conf_123")]
    pub transformation_id: String,
    #[schema(example = "tran_his_456")]
    pub transformation_history_id: Option<String>,
    #[schema(example = "order_123456")]
    pub order_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetStagingEntryResponse {
    #[schema(example = "stag_ent_123")]
    pub id: String,
    #[schema(example = "stag_ent_main_123")]
    pub staging_entry_id: String,
    pub account: AccountData,
    pub entry_type: BalanceDirection,
    pub amount: AmountResponse,
    pub status: ApiStagingEntryCoarseStatus,
    pub detailed_status: ApiStagingEntryStatus,
    pub processing_mode: StagingEntryProcessingMode,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    #[schema(value_type = serde_json::Value)]
    pub metadata: EncryptableValue,
    pub exposed_metadata: serde_json::Value,
    pub searchable_metadata: serde_json::Value,
    // TODO: Remove this after frontend starts using new transformation data structure
    pub transformation_id: String,
    pub transformation_config: ApiTransformationConfigData,
    #[schema(example = "tran_his_456")]
    pub transformation_history_id: Option<String>,
    pub data: StagingEntryData,
    #[schema(example = 1)]
    pub version: i32,
    #[schema(value_type = Option<String>, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<ApiStagingEntryCoarseStatus>,
    pub detailed_discarded_status: Option<ApiStagingEntryStatus>,
    pub discarded_data: Option<StagingEntryData>,
    #[schema(example = "order_123456")]
    pub order_id: String,
    /// User who modified the staging entry
    pub modified_by: Option<User>,
}

/// Simple data structure for API responses - flattened version of complex diesel model data
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StagingEntryData {
    Pending(PendingData),
    NeedsManualReview(NeedsManualReviewData),
    Processed,
    Void { reason: Option<String> },
    Archived,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "pending_type", rename_all = "snake_case")]
pub enum PendingData {
    New,
    Edited { reason: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "needs_manual_review_type", rename_all = "snake_case")]
pub enum NeedsManualReviewData {
    NoRulesFound,
    StagingEntryCurrencyMismatch {
        staging_currency: String,
        account_currency: String,
    },
    MissingSearchIdentifierValue {
        missing_field: EntryField,
    },
    DuplicateEntry,
    NoExpectationEntryFound,
    MultipleExpectedEntriesFound {
        transaction_ids: Vec<String>,
    },
    MissingMatchField {
        missing_field: EntryField,
    },
    MissingUniqueField {
        missing_field: EntryField,
    },
    MissingGroupingField {
        missing_field: EntryField,
    },
    InternalError,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VoidStagingEntryRequest {
    /// Reason for voiding the staging entry
    #[schema(example = "Customer requested cancellation")]
    pub reason: Option<String>,
}

/// Request for creating V2 staging entry with metadata schema validation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StagingEntryCreateRequestV2 {
    #[schema(example = "account_123")]
    pub account_id: String,
    #[schema(example = "schema_123")]
    pub metadata_schema_id: String,
    /// Raw field data to be validated against the metadata schema
    #[schema(value_type = HashMap<String, Option<String>>)]
    pub raw_data: HashMap<String, Option<String>>,
    #[schema(example = "tran_conf_123")]
    pub transformation_id: String,
    #[schema(example = "tran_his_456")]
    pub transformation_history_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StagingEntryUpdateRequest {
    #[schema(example = "Correcting amount due to error")]
    pub reason: String,
    #[schema(example = "account_123")]
    pub account_id: String,
    pub entry_type: BalanceDirection,
    #[schema(example = 99.99)]
    pub amount: FloatMajorUnit,
    pub currency: Currency,
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
    #[schema(value_type = serde_json::Value)]
    pub metadata: SecretValue,
    #[schema(example = "tran_conf_123")]
    pub transformation_id: String,
    #[schema(example = "order_123456")]
    pub order_id: String,
}

#[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct ApiListStagingEntriesFilters {
    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "pending,processed", value_type = Vec<ApiStagingEntryCoarseStatus>)]
    pub status: Option<Vec<ApiStagingEntryCoarseStatus>>,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "credit,debit", value_type = Vec<BalanceDirection>)]
    pub entry_type: Option<Vec<BalanceDirection>>,

    #[param(example = "acc_123")]
    pub account_id: Option<String>,

    #[param(example = "trans_his_123")]
    pub transformation_history_id: Option<String>,

    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(value_type = Vec<Currency>)]
    pub currency: Option<Vec<Currency>>,

    #[serde(flatten, default)]
    #[param(value_type = TimeRange)]
    pub time_range: Option<TimeRange>,

    #[param(example = "stag_ent_123")]
    pub staging_entry_id: Option<String>,
}

impl ApiListStagingEntriesFilters {
    pub fn validate(&self) -> Result<(), String> {
        self.time_range
            .map(|time_range| time_range.validate())
            .transpose()
            .map_err(|_| "start_time cannot be greater than end_time".to_string())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct ApiStagingEntryListFiltersV2 {
    // TODO: drop `status` once the frontend sends `detailed_status`.
    #[serde(default)]
    pub status: Option<Vec<ApiStagingEntryCoarseStatus>>,
    #[serde(default)]
    pub detailed_status: Option<Vec<ApiStagingEntryStatus>>,
    #[serde(default)]
    pub entry_type: Option<Vec<BalanceDirection>>,
    #[serde(default)]
    pub account_ids: Option<Vec<String>>,
    #[serde(default)]
    pub transformation_config_ids: Option<Vec<String>>,
    // TODO: remove singular `transformation_history_id` after clients migrate to `transformation_history_ids`.
    #[serde(default)]
    pub transformation_history_id: Option<String>,
    #[serde(default)]
    pub transformation_history_ids: Option<Vec<String>>,
    #[serde(default)]
    pub currency: Option<Vec<Currency>>,
    #[serde(default)]
    pub time_range: Option<TimeRange>,
    #[serde(default)]
    pub staging_entry_id: Option<String>,
    #[serde(default)]
    pub order_id: Option<String>,
}

impl ApiStagingEntryListFiltersV2 {
    pub fn validate(&self) -> Result<(), StagingEntryListFilterValidationError> {
        self.time_range
            .map(|time_range| time_range.validate())
            .transpose()
            .map_err(|_| StagingEntryListFilterValidationError::InvalidTimeRange)?;

        if let Some(status) = self
            .status
            .iter()
            .flatten()
            .find(|status| matches!(status, ApiStagingEntryCoarseStatus::Archived))
        {
            return Err(StagingEntryListFilterValidationError::UnsupportedStatus {
                status: *status,
            });
        }

        if let Some(status) = self
            .detailed_status
            .iter()
            .flatten()
            .find(|status| matches!(status, ApiStagingEntryStatus::Archived))
        {
            return Err(
                StagingEntryListFilterValidationError::UnsupportedDetailedStatus {
                    status: *status,
                },
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagingEntryListFilterValidationError {
    InvalidTimeRange,
    UnsupportedStatus { status: ApiStagingEntryCoarseStatus },
    UnsupportedDetailedStatus { status: ApiStagingEntryStatus },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ListStagingEntriesV2Request {
    #[serde(default)]
    pub filters: ApiStagingEntryListFiltersV2,
    pub cursor_payload: CursorPagination<StagingEntryListSortField>,
}

pub type StagingEntryListV2Page = CursorPage<GetStagingEntryResponse, StagingEntryListSortField>;

/// Client-controlled filters for the cursor-paginated linkable staging entry list.
///
/// Currency and status are intentionally absent because the server derives them from the
/// transaction and the linkability rules.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct ApiLinkableStagingEntryListFiltersV2 {
    #[serde(default)]
    pub entry_type: Option<Vec<BalanceDirection>>,
    #[serde(default)]
    pub account_ids: Option<Vec<String>>,
    #[serde(default)]
    pub transformation_config_ids: Option<Vec<String>>,
    #[serde(default)]
    pub transformation_history_ids: Option<Vec<String>>,
    #[serde(default)]
    pub time_range: Option<TimeRange>,
    #[serde(default)]
    pub staging_entry_id: Option<String>,
    #[serde(default)]
    pub order_id: Option<String>,
}

impl ApiLinkableStagingEntryListFiltersV2 {
    /// Validates filter combinations before querying linkable staging entries.
    pub fn validate(&self) -> Result<(), StagingEntryListFilterValidationError> {
        self.time_range
            .map(|time_range| time_range.validate())
            .transpose()
            .map_err(|_| StagingEntryListFilterValidationError::InvalidTimeRange)?;
        Ok(())
    }
}

/// Request for listing linkable staging entries with cursor pagination.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ListLinkableStagingEntriesV2Request {
    #[serde(default)]
    pub filters: ApiLinkableStagingEntryListFiltersV2,
    pub cursor_payload: CursorPagination<StagingEntryListSortField>,
}

/// Cursor-paginated linkable staging entry response.
pub type LinkableStagingEntryListV2Page =
    CursorPage<GetStagingEntryResponse, StagingEntryListSortField>;
