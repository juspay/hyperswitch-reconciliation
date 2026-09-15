use std::{collections::HashMap, path::PathBuf};

use crate::metadata_schema::{MetadataSchemaData, StringTransformationRule};
use common_enums::{BalanceDirection, Currency, TransformationStatus};
use common_types::{
    transformation::{TransformationConfigData, TransformationFailureStage},
    units::MinorUnit,
};
use common_utils::id_type::MerchantId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransformationReportFormat {
    Csv,
    Json,
}

/// Request for transformation execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransformationTriggerRequest {
    #[schema(value_type = String)]
    pub file_path: PathBuf,
}

/// Request for creating V2 transformation config with metadata schema
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTransformationConfigV2Request {
    pub ingestion_id: String,
    pub account_id: String,
    pub name: String,
    pub metadata_schema_data: MetadataSchemaData,
    pub config: TransformationConfigV2,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTransformationConfigRequest {
    #[schema(example = "Updated Transformation")]
    pub name: Option<String>,
    pub skip_configs: Option<Vec<SkipConfig>>,
    pub parsing_config: Option<ParsingConfig>,
    pub metadata_schema_data: Option<MetadataSchemaData>,
    #[schema(example = false)]
    pub is_active: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "transformation_result", rename_all = "snake_case")]
pub enum TransformationResultData {
    Processed {
        total_count: usize,
        transformed_count: usize,
        ignored_count: usize,
        failed_count: usize,
        #[schema(value_type = Option<String>)]
        csv_report_file_key: Option<PathBuf>,
        #[schema(value_type = Option<String>)]
        json_report_file_key: Option<PathBuf>,
        errors: Vec<String>,
    },
    Failed {
        failure_stage: TransformationFailureStage,
        error_message: String,
    },
}
/// Response for transformation history
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransformationHistoryResponse {
    pub transformation_history_id: String,
    pub ingestion_history_id: String,
    pub transformation_id: String,
    pub transformation_name: String,
    pub account_id: String,
    pub status: TransformationStatus,
    pub data: Option<TransformationResultData>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub processed_at: Option<time::PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
}

/// Response for transformation config
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransformationConfigResponse {
    pub transformation_id: String,
    pub profile_id: String,
    pub ingestion_id: String,
    pub account_id: String,
    pub name: String,
    pub config: TransformationConfigData,
    pub is_active: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_transformed_at: Option<time::PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: time::PrimitiveDateTime,
    pub metadata_schema_id: String,
}

/// Response for listing transformation configs
pub type TransformationConfigListResponse = Vec<TransformationConfigResponse>;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiTransformationConfigData {
    #[schema(example = "tran_conf_123")]
    pub transformation_config_id: String,
    pub transformation_config_name: String,
}

/// Response for listing transformation history
pub type TransformationHistoryListResponse = Vec<TransformationHistoryResponse>;

/// V2 transformation config API model - only V2 variants
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "transformation_type", rename_all = "snake_case")]
pub struct TransformationConfigV2 {
    #[schema(value_type = String)]
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub account_id: String,
    pub parsing_config: ParsingConfig,
    #[serde(default)]
    pub skip_configs: Vec<SkipConfig>,
}

/// Parsing config API model - flattened without versioning
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "file_format", rename_all = "snake_case")]
pub enum ParsingConfig {
    Csv(CsvParsingConfig),
    Xlsx(XlsxParsingConfig),
    FixedWidth(FixedWidthConfig),
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

/// CSV parsing config API model - simplified
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CsvParsingConfig;

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
pub struct FixedWidthConfig {
    pub fields: Vec<FixedWidthField>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FixedWidthField {
    pub header: String,
    pub index: usize,
    pub width: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SkipConfig {
    pub conditions: Vec<SkipCondition>,
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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ValidateEntryDataRequest {
    #[schema(example = "tran_conf_123")]
    pub transformation_id: String,
    pub raw_entry_data: HashMap<String, Option<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ValidateEntryDataResponse {
    Success(ValidatedEntryData),
    Failure(RowError),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ValidatedEntryData {
    pub entry_type: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub effective_at: time::PrimitiveDateTime,
    pub order_id: String,
    pub metadata: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RowError {
    pub errors: HashMap<String, String>,
}
