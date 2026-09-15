use crate::{
    recon_rule::entry_field::EntryField,
    staging_entry::status::{DomainStagingEntryStatus, StagingEntryManualReviewData},
};
use common_enums::DieselStagingEntryStatus;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "data_version", rename_all = "snake_case")]
pub enum StagingEntryData {
    V2(StagingEntryDataV2),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StagingEntryDataV2 {
    Pending(PendingData),
    NeedsManualReview(ManualReviewData),
    Processed,
    Void(VoidData),
    Archived,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "pending_data_version", rename_all = "snake_case")]
pub enum PendingData {
    V1(PendingDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "pending_data_type", rename_all = "snake_case")]
pub enum PendingDataV1 {
    New,
    Edited(EditedData),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "manual_review_data_version", rename_all = "snake_case")]
pub enum ManualReviewData {
    V1(ManualReviewDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ManualReviewDataV1 {
    pub error_type: ProcessingErrorType,
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
#[serde(tag = "edited_data_version", rename_all = "snake_case")]
pub enum EditedData {
    V1(EditedDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EditedDataV1 {
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "error_version", rename_all = "snake_case")]
pub enum ProcessingErrorType {
    V1(ProcessingErrorTypeV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "error_type", rename_all = "snake_case")]
pub enum ProcessingErrorTypeV1 {
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

impl StagingEntryData {
    pub fn is_duplicate(&self) -> bool {
        match self {
            Self::V2(v2_data) => v2_data.is_duplicate(),
        }
    }
}

impl StagingEntryDataV2 {
    pub fn is_duplicate(&self) -> bool {
        match self {
            Self::NeedsManualReview(manual_review_data) => manual_review_data.is_duplicate(),
            Self::Pending(_) | Self::Processed | Self::Void(_) | Self::Archived => false,
        }
    }
}

impl ManualReviewData {
    pub fn is_duplicate(&self) -> bool {
        match self {
            Self::V1(v1_data) => v1_data.error_type.is_duplicate(),
        }
    }
}

impl ProcessingErrorType {
    pub fn is_duplicate(&self) -> bool {
        match self {
            Self::V1(ProcessingErrorTypeV1::DuplicateEntry) => true,
            Self::V1(ProcessingErrorTypeV1::NoRulesFound)
            | Self::V1(ProcessingErrorTypeV1::StagingEntryCurrencyMismatch { .. })
            | Self::V1(ProcessingErrorTypeV1::MissingSearchIdentifierValue { .. })
            | Self::V1(ProcessingErrorTypeV1::NoExpectationEntryFound)
            | Self::V1(ProcessingErrorTypeV1::MultipleExpectedEntriesFound { .. })
            | Self::V1(ProcessingErrorTypeV1::MissingMatchField { .. })
            | Self::V1(ProcessingErrorTypeV1::MissingUniqueField { .. })
            | Self::V1(ProcessingErrorTypeV1::MissingGroupingField { .. })
            | Self::V1(ProcessingErrorTypeV1::InternalError) => false,
        }
    }
}

common_utils::impl_to_sql_from_sql_json!(StagingEntryData);

impl From<&StagingEntryData> for DomainStagingEntryStatus {
    fn from(data: &StagingEntryData) -> Self {
        match data {
            StagingEntryData::V2(v2_data) => v2_data.into(),
        }
    }
}

impl From<&StagingEntryDataV2> for DomainStagingEntryStatus {
    fn from(data: &StagingEntryDataV2) -> Self {
        match data {
            StagingEntryDataV2::Pending(_) => Self::Pending,
            StagingEntryDataV2::NeedsManualReview(manual_review_data) => {
                Self::NeedsManualReview(manual_review_data.into())
            }
            StagingEntryDataV2::Processed => Self::Processed,
            StagingEntryDataV2::Archived => Self::Archived,
            StagingEntryDataV2::Void(_) => Self::Void,
        }
    }
}

impl From<&ManualReviewData> for StagingEntryManualReviewData {
    fn from(value: &ManualReviewData) -> Self {
        match value {
            ManualReviewData::V1(manual_review_data_v1) => manual_review_data_v1.into(),
        }
    }
}

impl From<&ManualReviewDataV1> for StagingEntryManualReviewData {
    fn from(value: &ManualReviewDataV1) -> Self {
        (&value.error_type).into()
    }
}

impl From<&ProcessingErrorType> for StagingEntryManualReviewData {
    fn from(value: &ProcessingErrorType) -> Self {
        match value {
            ProcessingErrorType::V1(error_v1) => error_v1.into(),
        }
    }
}

impl From<&ProcessingErrorTypeV1> for StagingEntryManualReviewData {
    fn from(value: &ProcessingErrorTypeV1) -> Self {
        match value {
            ProcessingErrorTypeV1::NoRulesFound => Self::NoRulesFound,
            ProcessingErrorTypeV1::StagingEntryCurrencyMismatch { .. } => Self::CurrencyMismatch,
            ProcessingErrorTypeV1::MissingSearchIdentifierValue { .. } => {
                Self::MissingSearchIdentifierValue
            }
            ProcessingErrorTypeV1::DuplicateEntry => Self::DuplicateEntry,
            ProcessingErrorTypeV1::NoExpectationEntryFound => Self::NoExpectationEntryFound,
            ProcessingErrorTypeV1::MultipleExpectedEntriesFound { .. } => {
                Self::MultipleExpectedEntriesFound
            }
            ProcessingErrorTypeV1::MissingMatchField { .. } => Self::MissingMatchField,
            ProcessingErrorTypeV1::MissingUniqueField { .. } => Self::MissingUniqueField,
            ProcessingErrorTypeV1::MissingGroupingField { .. } => Self::MissingGroupingField,
            ProcessingErrorTypeV1::InternalError => Self::InternalError,
        }
    }
}

impl From<&StagingEntryData> for DieselStagingEntryStatus {
    fn from(value: &StagingEntryData) -> Self {
        DomainStagingEntryStatus::from(value).into()
    }
}
