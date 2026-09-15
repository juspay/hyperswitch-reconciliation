use common_enums::DieselStagingEntryStatus;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", content = "sub_status", rename_all = "snake_case")]
pub enum DomainStagingEntryStatus {
    Pending,
    NeedsManualReview(StagingEntryManualReviewData),
    Processed,
    Void,
    Archived,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize, EnumIter, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StagingEntryManualReviewData {
    NoRulesFound,
    CurrencyMismatch,
    MissingSearchIdentifierValue,
    DuplicateEntry,
    NoExpectationEntryFound,
    MultipleExpectedEntriesFound,
    MissingMatchField,
    MissingUniqueField,
    MissingGroupingField,
    InternalError,
}

impl From<DomainStagingEntryStatus> for DieselStagingEntryStatus {
    fn from(value: DomainStagingEntryStatus) -> Self {
        match value {
            DomainStagingEntryStatus::Pending => Self::Pending,
            DomainStagingEntryStatus::NeedsManualReview(manual_review_data) => {
                match manual_review_data {
                    StagingEntryManualReviewData::NoRulesFound => {
                        Self::NeedsManualReviewNoRulesFound
                    }
                    StagingEntryManualReviewData::CurrencyMismatch => {
                        Self::NeedsManualReviewCurrencyMismatch
                    }
                    StagingEntryManualReviewData::MissingSearchIdentifierValue => {
                        Self::NeedsManualReviewMissingSearchIdentifierValue
                    }
                    StagingEntryManualReviewData::DuplicateEntry => {
                        Self::NeedsManualReviewDuplicateEntry
                    }
                    StagingEntryManualReviewData::NoExpectationEntryFound => {
                        Self::NeedsManualReviewNoExpectationEntryFound
                    }
                    StagingEntryManualReviewData::MultipleExpectedEntriesFound => {
                        Self::NeedsManualReviewMultipleExpectedEntriesFound
                    }
                    StagingEntryManualReviewData::MissingMatchField => {
                        Self::NeedsManualReviewMissingMatchField
                    }
                    StagingEntryManualReviewData::MissingUniqueField => {
                        Self::NeedsManualReviewMissingUniqueField
                    }
                    StagingEntryManualReviewData::MissingGroupingField => {
                        Self::NeedsManualReviewMissingGroupingField
                    }
                    StagingEntryManualReviewData::InternalError => {
                        Self::NeedsManualReviewInternalError
                    }
                }
            }
            DomainStagingEntryStatus::Processed => Self::Processed,
            DomainStagingEntryStatus::Void => Self::Void,
            DomainStagingEntryStatus::Archived => Self::Archived,
        }
    }
}

impl From<DieselStagingEntryStatus> for DomainStagingEntryStatus {
    fn from(value: DieselStagingEntryStatus) -> Self {
        match value {
            DieselStagingEntryStatus::Pending => Self::Pending,
            DieselStagingEntryStatus::NeedsManualReviewNoRulesFound => {
                Self::NeedsManualReview(StagingEntryManualReviewData::NoRulesFound)
            }
            DieselStagingEntryStatus::NeedsManualReviewCurrencyMismatch => {
                Self::NeedsManualReview(StagingEntryManualReviewData::CurrencyMismatch)
            }
            DieselStagingEntryStatus::NeedsManualReviewMissingSearchIdentifierValue => {
                Self::NeedsManualReview(StagingEntryManualReviewData::MissingSearchIdentifierValue)
            }
            DieselStagingEntryStatus::NeedsManualReviewDuplicateEntry => {
                Self::NeedsManualReview(StagingEntryManualReviewData::DuplicateEntry)
            }
            DieselStagingEntryStatus::NeedsManualReviewNoExpectationEntryFound => {
                Self::NeedsManualReview(StagingEntryManualReviewData::NoExpectationEntryFound)
            }
            DieselStagingEntryStatus::NeedsManualReviewMultipleExpectedEntriesFound => {
                Self::NeedsManualReview(StagingEntryManualReviewData::MultipleExpectedEntriesFound)
            }
            DieselStagingEntryStatus::NeedsManualReviewMissingMatchField => {
                Self::NeedsManualReview(StagingEntryManualReviewData::MissingMatchField)
            }
            DieselStagingEntryStatus::NeedsManualReviewMissingUniqueField => {
                Self::NeedsManualReview(StagingEntryManualReviewData::MissingUniqueField)
            }
            DieselStagingEntryStatus::NeedsManualReviewMissingGroupingField => {
                Self::NeedsManualReview(StagingEntryManualReviewData::MissingGroupingField)
            }
            DieselStagingEntryStatus::NeedsManualReviewInternalError => {
                Self::NeedsManualReview(StagingEntryManualReviewData::InternalError)
            }
            DieselStagingEntryStatus::Processed => Self::Processed,
            DieselStagingEntryStatus::Void => Self::Void,
            DieselStagingEntryStatus::Archived => Self::Archived,
        }
    }
}

impl StagingEntryManualReviewData {
    pub fn is_linkable(self) -> bool {
        match self {
            Self::NoRulesFound
            | Self::MissingSearchIdentifierValue
            | Self::DuplicateEntry
            | Self::NoExpectationEntryFound
            | Self::MultipleExpectedEntriesFound
            | Self::MissingMatchField => true,
            Self::CurrencyMismatch
            | Self::MissingUniqueField
            | Self::MissingGroupingField
            | Self::InternalError => false,
        }
    }
}

impl DomainStagingEntryStatus {
    pub fn is_linkable(self) -> bool {
        match self {
            Self::NeedsManualReview(manual_review_data) => manual_review_data.is_linkable(),
            Self::Pending | Self::Processed | Self::Void | Self::Archived => false,
        }
    }

    pub fn is_voidable(self) -> bool {
        match self {
            Self::Pending | Self::NeedsManualReview(_) => true,
            Self::Processed | Self::Void | Self::Archived => false,
        }
    }
}
