use common_enums::DieselEntryLinkingStatus;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use utoipa::ToSchema;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize, ToSchema, strum::Display,
)]
#[serde(tag = "status", content = "sub_status", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DomainEntryLinkingStatus {
    Pending,
    Processed,
    Failed(EntryLinkingFailureReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize, EnumIter, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntryLinkingFailureReason {
    Expected,
    InternalError,
    NoTransformationConfigFound,
    NoLinkConfigFound,
    MissingSearchIdentifierValue,
    MultiplePostedLinksFound,
}

impl From<DomainEntryLinkingStatus> for DieselEntryLinkingStatus {
    fn from(value: DomainEntryLinkingStatus) -> Self {
        match value {
            DomainEntryLinkingStatus::Pending => Self::Pending,
            DomainEntryLinkingStatus::Processed => Self::Processed,
            DomainEntryLinkingStatus::Failed(reason) => match reason {
                EntryLinkingFailureReason::Expected => Self::FailedExpectedEntry,
                EntryLinkingFailureReason::InternalError => Self::FailedInternalError,
                EntryLinkingFailureReason::NoTransformationConfigFound => {
                    Self::FailedNoTransformationConfigFound
                }
                EntryLinkingFailureReason::NoLinkConfigFound => Self::FailedNoLinkConfigFound,
                EntryLinkingFailureReason::MissingSearchIdentifierValue => {
                    Self::FailedMissingSearchIdentifierValue
                }
                EntryLinkingFailureReason::MultiplePostedLinksFound => {
                    Self::FailedMultiplePostedLinksFound
                }
            },
        }
    }
}

impl From<DieselEntryLinkingStatus> for DomainEntryLinkingStatus {
    fn from(value: DieselEntryLinkingStatus) -> Self {
        match value {
            DieselEntryLinkingStatus::Pending => Self::Pending,
            DieselEntryLinkingStatus::Processed => Self::Processed,
            DieselEntryLinkingStatus::FailedExpectedEntry => {
                Self::Failed(EntryLinkingFailureReason::Expected)
            }
            DieselEntryLinkingStatus::FailedInternalError => {
                Self::Failed(EntryLinkingFailureReason::InternalError)
            }
            DieselEntryLinkingStatus::FailedNoTransformationConfigFound => {
                Self::Failed(EntryLinkingFailureReason::NoTransformationConfigFound)
            }
            DieselEntryLinkingStatus::FailedNoLinkConfigFound => {
                Self::Failed(EntryLinkingFailureReason::NoLinkConfigFound)
            }
            DieselEntryLinkingStatus::FailedMissingSearchIdentifierValue => {
                Self::Failed(EntryLinkingFailureReason::MissingSearchIdentifierValue)
            }
            DieselEntryLinkingStatus::FailedMultiplePostedLinksFound => {
                Self::Failed(EntryLinkingFailureReason::MultiplePostedLinksFound)
            }
        }
    }
}

impl DomainEntryLinkingStatus {
    pub fn enqueuable_statuses() -> Vec<Self> {
        EntryLinkingFailureReason::iter()
            .map(Self::Failed)
            .collect()
    }
}
