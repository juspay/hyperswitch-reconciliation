use common_enums::EntryLinkingStatus;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::EntryField;

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
    NotApplicable,
}

impl From<&EntryLinkingData> for EntryLinkingStatus {
    fn from(data: &EntryLinkingData) -> Self {
        match data {
            EntryLinkingData::V1(data) => data.into(),
        }
    }
}

impl From<&EntryLinkingDataV1> for EntryLinkingStatus {
    fn from(data: &EntryLinkingDataV1) -> Self {
        match data {
            EntryLinkingDataV1::Pending => Self::Pending,
            EntryLinkingDataV1::Processed => Self::Processed,
            EntryLinkingDataV1::Failed(_) => Self::Failed,
            EntryLinkingDataV1::NotApplicable => Self::NotApplicable,
        }
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
    InternalError { message: String },
    TransformationConfigMissing,
    NoLinkConfigFound,
    MissingSearchIdentifierValue { missing_field: EntryField },
    MultiplePostedLinksFound,
}
