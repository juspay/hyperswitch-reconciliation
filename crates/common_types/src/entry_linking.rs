use common_enums::DieselEntryLinkingStatus;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::EntryField;
use status::{DomainEntryLinkingStatus, EntryLinkingFailureReason};

pub mod status;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow, PartialEq, Eq,
)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "data_version", rename_all = "snake_case")]
pub enum EntryLinkingData {
    V1(EntryLinkingDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EntryLinkingDataV1 {
    Pending,
    Processed,
    Failed(LinkingFailureData),
}

impl From<&EntryLinkingData> for DomainEntryLinkingStatus {
    fn from(data: &EntryLinkingData) -> Self {
        match data {
            EntryLinkingData::V1(data) => data.into(),
        }
    }
}

impl From<&EntryLinkingDataV1> for DomainEntryLinkingStatus {
    fn from(data: &EntryLinkingDataV1) -> Self {
        match data {
            EntryLinkingDataV1::Pending => Self::Pending,
            EntryLinkingDataV1::Processed => Self::Processed,
            EntryLinkingDataV1::Failed(LinkingFailureData::V1(data)) => {
                let LinkingProcessingErrorType::V1(error_type) = &data.error_type;
                Self::Failed(match error_type {
                    LinkingProcessingErrorTypeV1::Expected => EntryLinkingFailureReason::Expected,
                    LinkingProcessingErrorTypeV1::InternalError { .. } => {
                        EntryLinkingFailureReason::InternalError
                    }
                    LinkingProcessingErrorTypeV1::NoTransformationConfigFound => {
                        EntryLinkingFailureReason::NoTransformationConfigFound
                    }
                    LinkingProcessingErrorTypeV1::NoLinkConfigFound => {
                        EntryLinkingFailureReason::NoLinkConfigFound
                    }
                    LinkingProcessingErrorTypeV1::MissingSearchIdentifierValue { .. } => {
                        EntryLinkingFailureReason::MissingSearchIdentifierValue
                    }
                    LinkingProcessingErrorTypeV1::MultiplePostedLinksFound => {
                        EntryLinkingFailureReason::MultiplePostedLinksFound
                    }
                })
            }
        }
    }
}

impl From<&EntryLinkingData> for DieselEntryLinkingStatus {
    fn from(data: &EntryLinkingData) -> Self {
        DomainEntryLinkingStatus::from(data).into()
    }
}

common_utils::impl_to_sql_from_sql_json!(EntryLinkingData);

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "failure_data_version", rename_all = "snake_case")]
pub enum LinkingFailureData {
    V1(LinkingFailureDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct LinkingFailureDataV1 {
    pub error_type: LinkingProcessingErrorType,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "error_type_version", rename_all = "snake_case")]
pub enum LinkingProcessingErrorType {
    V1(LinkingProcessingErrorTypeV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "error_type", rename_all = "snake_case")]
pub enum LinkingProcessingErrorTypeV1 {
    Expected,
    InternalError { message: String },
    NoTransformationConfigFound,
    NoLinkConfigFound,
    MissingSearchIdentifierValue { missing_field: EntryField },
    MultiplePostedLinksFound,
}
