use std::path::PathBuf;

use crate::metadata_schema::datatypes::string::StringTransformationRule;
use crate::{date_time::TimeRange, transformation::archived::TransformationDataV1};
use common_enums::TransformationStatus;
use common_utils::custom_serde::query_parsers::parse_comma_separated;
use common_utils::id_type::MerchantId;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub mod archived;

/// Stages where transformation can fail after history entry is created
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransformationFailureStage {
    FileRetrieval,
    FileParsing,
    TransformationExecution,
    TransformationReportUpload,
    StagingEntryInsertion,
}

/// Transformation config with versioning
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum TransformationConfigData {
    V2(TransformationConfigDataV2),
}

/// V2 Transformation configuration with metadata schema integration
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TransformationConfigDataV2 {
    #[schema(value_type = String)]
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub account_id: String,
    pub parsing_config: ParsingConfig,
    #[serde(default)]
    pub skip_configs: Vec<SkipConfig>,
}

/// Transformation data with success/failure variants
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum TransformationData {
    V1(TransformationDataV1),
    V2(TransformationDataV2),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TransformationDataV2 {
    Processed {
        #[schema(value_type = String)]
        csv_report_file_key: PathBuf,
        #[schema(value_type = String)]
        json_report_file_key: PathBuf,
        success_count: usize,
        failed_count: usize,
        ignored_count: usize,
    },
    Failed {
        failure_stage: TransformationFailureStage,
        error_message: String,
    },
}

// Use the macro from common_utils to implement JSON serialization/deserialization
common_utils::impl_to_sql_from_sql_json!(TransformationConfigData);
common_utils::impl_to_sql_from_sql_json!(TransformationData);

/// Filters for listing transformation configs
#[derive(Debug, Clone, Default, Deserialize, IntoParams, ToSchema)]
pub struct ListTransformationConfigsFilters {
    /// Filter by ingestion_id
    #[param(example = "ing_123")]
    pub ingestion_id: Option<String>,

    /// Filter by account_id
    #[param(example = "acc_123")]
    pub account_id: Option<String>,

    /// Filter by active status
    #[param(example = "true")]
    pub is_active: Option<bool>,

    /// Filter by time range (created_at)
    #[serde(flatten, default)]
    #[param(value_type = TimeRange)]
    pub time_range: Option<TimeRange>,
}

/// Filters for listing transformation history
#[derive(Debug, Clone, Default, Deserialize, IntoParams, ToSchema)]
pub struct ListTransformationHistoryFilters {
    /// Filter by transformation_id (config ID)
    #[param(example = "tran_conf_123")]
    pub transformation_id: Option<String>,

    /// Filter by ingestion_history_id
    #[param(example = "ing_hist_123")]
    pub ingestion_history_id: Option<String>,

    /// Filter by account_id
    #[param(example = "acc_123")]
    pub account_id: Option<String>,

    /// Filter by transformation status
    #[serde(default, deserialize_with = "parse_comma_separated")]
    #[param(example = "processing,processed,failed", value_type = Vec<TransformationStatus>)]
    pub status: Option<Vec<TransformationStatus>>,

    /// Filter by time range (created_at)
    #[serde(flatten, default)]
    #[param(value_type = TimeRange)]
    pub time_range: Option<TimeRange>,
}

impl ListTransformationConfigsFilters {
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

impl ListTransformationHistoryFilters {
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

// ======== Parsing Configurations ========

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "parsing_config_version", rename_all = "snake_case")]
pub enum ParsingConfig {
    V1(ParsingConfigV1),
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "file_format", rename_all = "snake_case")]
pub enum ParsingConfigV1 {
    Csv(CsvParsingConfig),
    Xlsx(XlsxParsingConfig),
    FixedWidth(FixedWidthConfig),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "csv_parsing_config_version", rename_all = "snake_case")]
pub enum CsvParsingConfig {
    V1(CsvParsingConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CsvParsingConfigV1;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "xlsx_parsing_config_version", rename_all = "snake_case")]
pub enum XlsxParsingConfig {
    V1(XlsxParsingConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct XlsxParsingConfigV1 {
    pub header_row_index: usize,
    pub sheet_selection: SheetSelection,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(
    tag = "sheet_selection_type",
    content = "value",
    rename_all = "snake_case"
)]
pub enum SheetSelection {
    ByIndex(usize),
    ByName(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "fixed_width_parsing_config_version", rename_all = "snake_case")]
pub enum FixedWidthConfig {
    V1(FixedWidthConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FixedWidthConfigV1 {
    pub fields: Vec<FixedWidthField>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FixedWidthField {
    pub header: String,
    pub index: usize,
    pub width: usize,
}

// ==== Skip Configurations ====

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "skip_config_version", rename_all = "snake_case")]
pub enum SkipConfig {
    V1(SkipConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SkipConfigV1 {
    pub conditions: Vec<SkipCondition>, // `and` logic between conditions
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "skip_type", rename_all = "snake_case")]
pub enum SkipCondition {
    RowSkip(RowSkipConfig),
    ValueSkip(ValueSkipConfig),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "row_skip_type", rename_all = "snake_case")]
pub enum RowSkipConfig {
    LineNo { line_number: usize },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ValueSkipConfig {
    pub identifier: String,
    pub operator: SkipOperator,
    pub value: Option<String>,
    #[serde(default)]
    pub transformation_rules: Vec<StringTransformationRule>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkipOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    NotStartsWith,
    EndsWith,
    NotEndsWith,
}
