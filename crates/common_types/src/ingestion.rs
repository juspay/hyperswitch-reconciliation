use std::path::PathBuf;

use crate::date_time::TimeRange;
use adyen::AdyenTriggerData;
use common_enums::IngestionStatus;
use common_utils::custom_serde::query_parsers::parse_comma_separated;
use common_utils::PrimitiveDateTime;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use manual::ManualTriggerData;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub mod adyen;
pub mod manual;

/// Data stored in ingestion history with success/failure variants
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "ingestion_data_version", rename_all = "snake_case")]
pub enum IngestionData {
    V1(IngestionDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IngestionDataV1 {
    Processed(IngestionProcessedData),
    Failed(IngestionFailureData),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct IngestionProcessedData {
    /// S3 or filesystem path where the file is stored
    #[schema(value_type = String, example = "/path/to/ingested/file.csv")]
    pub file_key: PathBuf,
    /// When the file was successfully downloaded
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub downloaded_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct IngestionFailureData {
    /// Which stage the failure occurred at
    pub stage: IngestionFailureStage,
    /// Human-readable error message
    pub error_message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IngestionFailureStage {
    /// Failed during parsing the Trigger Data
    Parse,
    /// Failed during file download from external source
    Download,
    /// Failed during upload to storage (S3/filesystem)
    Storage,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "trigger_data_version", rename_all = "snake_case")]
pub enum TriggerData {
    V1(TriggerDataV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub enum TriggerDataV1 {
    Adyen(AdyenTriggerData),
    Manual(ManualTriggerData),
    SftpInternal,
}

/// Query parameters for listing ingestion configs
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListIngestionConfigsFilters {
    /// Filter by account_id
    #[param(value_type = String)]
    pub account_id: Option<String>,

    /// Filter configs based on their active status
    #[param(value_type = bool)]
    pub is_active: Option<bool>,

    /// TimeRange for filtering configs by creation date
    #[serde(flatten)]
    pub time_range: Option<TimeRange>,
}

/// Query parameters for listing ingestion history
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListIngestionHistoryFilters {
    /// Filter by account_id
    pub account_id: Option<String>,

    /// Filter by ingestion_history_id to get all versions of a specific business record
    pub ingestion_history_id: Option<String>,

    /// Filter by ingestion status
    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "pending,processed", value_type = Vec<IngestionStatus>)]
    pub status: Option<Vec<IngestionStatus>>,

    pub ingestion_id: Option<String>,

    /// TimeRange for filtering configs by creation date
    #[serde(flatten)]
    pub time_range: Option<TimeRange>,
}

impl ListIngestionConfigsFilters {
    /// Validates the time range filter
    pub fn validate(&self) -> Result<(), String> {
        if let Some(time_range) = &self.time_range {
            if let Some(end_time) = time_range.end_time {
                if time_range.start_time > end_time {
                    return Err("start_time cannot be greater than end_time".to_string());
                }
            }
        }
        Ok(())
    }
}

impl ListIngestionHistoryFilters {
    /// Validates the time range filter
    pub fn validate(&self) -> Result<(), String> {
        if let Some(time_range) = &self.time_range {
            if let Some(end_time) = time_range.end_time {
                if time_range.start_time > end_time {
                    return Err("start_time cannot be greater than end_time".to_string());
                }
            }
        }
        Ok(())
    }
}

/// Raw payload data stored in ingestion history for different ingestion types
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "raw_payload_version", rename_all = "snake_case")]
pub enum RawPayload {
    V1(RawPayloadV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "ingestion_type", rename_all = "snake_case")]
pub enum RawPayloadV1 {
    Adyen(serde_json::Value),
    Manual { file_name: String },
    SftpInternal,
}

common_utils::impl_to_sql_from_sql_json!(IngestionData);
common_utils::impl_to_sql_from_sql_json!(TriggerData);
common_utils::impl_to_sql_from_sql_json!(RawPayload);
