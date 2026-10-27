use std::path::PathBuf;

use serde::Serialize;

use crate::domain::{
    metadata_schema::skip_config::MatchedCondition,
    transformation::types::{
        RowFailureStage, RowResultOutcome, SourceLocation, SourceRecord, TransformationDiagnostic,
        TransformationRowResult,
    },
};
use api_models::transformation::{
    CreateTransformationConfigV2Request, CsvParsingConfig as CsvParsingConfigApi,
    FixedWidthConfig as FixedWidthConfigApi, FixedWidthField as FixedWidthFieldApi,
    ParsingConfig as ParsingConfigApi, RowSkipConfig as RowSkipConfigApi,
    SheetSelection as SheetSelectionApi, SkipCondition as SkipConditionApi,
    SkipConfig as SkipConfigApi, SkipOperator as SkipOperatorApi, TransformationConfigResponse,
    TransformationHistoryResponse, TransformationResultData, ValueSkipConfig as ValueSkipConfigApi,
    XlsxParsingConfig as XlsxParsingConfigApi,
};
use api_models::{
    metadata_schema::StringTransformationRule as StringTransformationRuleApi,
    transformation::TransformationReportFormat,
};
use common_types::{
    metadata_schema::datatypes::string::{StringTransformationRule, StringTransformationRuleV1},
    transformation::{
        archived::TransformationDataV1, CsvParsingConfig as DieselCsvParsingConfig,
        CsvParsingConfigV1, FixedWidthConfig as DieselFixedWidthConfig,
        FixedWidthConfigV1 as DieselFixedWidthConfigV1, FixedWidthField as DieselFixedWidthField,
        ParsingConfig as DieselParsingConfig, ParsingConfigV1, RowSkipConfig, SheetSelection,
        SkipCondition, SkipConfig, SkipConfigV1, SkipOperator, TransformationData,
        TransformationDataV2, TransformationFailureStage, ValueSkipConfig,
        XlsxParsingConfig as DieselXlsxParsingConfig,
        XlsxParsingConfigV1 as DieselXlsxParsingConfigV1,
    },
};
use diesel_models::{
    transformation_config::TransformationConfig,
    transformation_history::{TransformationConfigHistoryView, TransformationHistoryUpdate},
};
use error_stack::{report, ResultExt};
use storage_impl::transformation::TransformationHistoryInterface;

use crate::{
    errors::transformation::{TransformationError, TransformationResult},
    routes::AppState,
};

/// Trait for converting transformation config models to responses
pub trait TransformationConfigExt {
    /// Convert TransformationConfig database model to TransformationConfigResponse API model
    fn to_response(
        self,
        last_transformed_at: Option<time::PrimitiveDateTime>,
    ) -> TransformationConfigResponse;
}

/// Trait for converting transformation data to API result data format
pub trait TransformationDataApiExt {
    /// Convert TransformationData to TransformationResultData for API responses
    fn get_result_data(self) -> TransformationResultData;

    fn get_report_file_key(&self, report_format: TransformationReportFormat) -> Option<PathBuf>;
}

/// Implementation for TransformationConfig
impl TransformationConfigExt for TransformationConfig {
    fn to_response(
        self,
        last_transformed_at: Option<time::PrimitiveDateTime>,
    ) -> TransformationConfigResponse {
        TransformationConfigResponse {
            transformation_id: self.id,
            profile_id: self.profile_id,
            ingestion_id: self.ingestion_config_id,
            account_id: self.account_id,
            name: self.name,
            config: self.config,
            is_active: self.is_active,
            created_at: self.created_at,
            last_transformed_at,
            last_modified_at: self.last_modified_at,
            metadata_schema_id: self.metadata_schema_id,
        }
    }
}

/// Implementation for TransformationData
impl TransformationDataApiExt for TransformationData {
    fn get_result_data(self) -> TransformationResultData {
        match self {
            Self::V1(transformation_data_v1) => match transformation_data_v1 {
                TransformationDataV1::Processed(processed_data) => {
                    TransformationResultData::Processed {
                        total_count: processed_data.total_count,
                        transformed_count: processed_data.transformed_count,
                        ignored_count: processed_data.ignored_count,
                        failed_count: processed_data.errors.len(),
                        csv_report_file_key: None,
                        json_report_file_key: None,
                        errors: processed_data.errors,
                    }
                }
                TransformationDataV1::Failed(failure_data) => TransformationResultData::Failed {
                    failure_stage: failure_data.failure_stage,
                    error_message: failure_data.error_message,
                },
            },
            Self::V2(transformation_data_v2) => match transformation_data_v2 {
                TransformationDataV2::Processed {
                    csv_report_file_key,
                    json_report_file_key,
                    success_count,
                    failed_count,
                    ignored_count,
                } => TransformationResultData::Processed {
                    total_count: success_count + failed_count + ignored_count,
                    transformed_count: success_count,
                    ignored_count,
                    failed_count,
                    csv_report_file_key: Some(csv_report_file_key),
                    json_report_file_key: Some(json_report_file_key),
                    errors: Vec::new(),
                },
                TransformationDataV2::Failed {
                    failure_stage,
                    error_message,
                } => TransformationResultData::Failed {
                    failure_stage,
                    error_message,
                },
            },
        }
    }

    fn get_report_file_key(&self, report_format: TransformationReportFormat) -> Option<PathBuf> {
        match self {
            Self::V1(_) => None,
            Self::V2(TransformationDataV2::Processed {
                csv_report_file_key,
                json_report_file_key,
                ..
            }) => match report_format {
                TransformationReportFormat::Csv => Some(csv_report_file_key.clone()),
                TransformationReportFormat::Json => Some(json_report_file_key.clone()),
            },
            Self::V2(TransformationDataV2::Failed { .. }) => None,
        }
    }
}

/// Update transformation history to failed status with error details
pub async fn update_transformation_history_failed(
    state: &AppState,
    transformation_history_id: &str,
    failure_stage: TransformationFailureStage,
    error_message: &str,
) -> TransformationResult<()> {
    let transformation_data = TransformationData::V2(TransformationDataV2::Failed {
        failure_stage,
        error_message: error_message.to_string(),
    });

    state
        .store
        .update_transformation_history_by_id(
            transformation_history_id,
            TransformationHistoryUpdate::Failed {
                failure_data: transformation_data,
            },
        )
        .await
        .change_context(TransformationError::InternalServerError)
        .map(|_| ())
}

pub trait TransformationConfigHistoryViewExt {
    fn to_api_response(self) -> TransformationHistoryResponse;
}

impl TransformationConfigHistoryViewExt for TransformationConfigHistoryView {
    fn to_api_response(self) -> TransformationHistoryResponse {
        let data = self.history.data.map(|data| data.get_result_data());

        TransformationHistoryResponse {
            transformation_history_id: self.history.id,
            ingestion_history_id: self.history.ingestion_history_id,
            transformation_id: self.history.transformation_config_id,
            account_id: self.history.account_id,
            status: self.history.status,
            data,
            transformation_name: self.config.name,
            processed_at: self.history.processed_at,
            created_at: self.history.created_at,
        }
    }
}

/// Trait for converting API parsing types to common types
pub trait ToDiesel<T> {
    fn to_diesel(self) -> T;
}

#[derive(Serialize)]
struct ReadableTransformationReportRow {
    source_type: &'static str,
    identifier: String,
    outcome: &'static str,
    details: String,
}

impl From<TransformationRowResult> for ReadableTransformationReportRow {
    fn from(row: TransformationRowResult) -> Self {
        let (source_type, identifier) = format_report_source(&row.source);
        let (outcome, details) = match &row.outcome {
            RowResultOutcome::Transformed { staging_entry_id } => (
                "transformed",
                format!("Created staging entry {staging_entry_id}"),
            ),
            RowResultOutcome::Skipped { reason } => (
                "skipped",
                format_matched_conditions(&reason.matched_conditions),
            ),
            RowResultOutcome::Failed { stage, errors } => {
                ("failed", format_failed_details(*stage, errors))
            }
        };

        Self {
            source_type,
            identifier,
            outcome,
            details,
        }
    }
}

pub async fn generate_and_upload_transformation_reports(
    state: &AppState,
    transformation_id: &str,
    transformation_history_id: &str,
    rows: Vec<TransformationRowResult>,
) -> TransformationResult<TransformationData> {
    let success_count = rows
        .iter()
        .filter(|row| matches!(&row.outcome, RowResultOutcome::Transformed { .. }))
        .count();
    let ignored_count = rows
        .iter()
        .filter(|row| matches!(&row.outcome, RowResultOutcome::Skipped { .. }))
        .count();
    let failed_count = rows
        .iter()
        .filter(|row| matches!(&row.outcome, RowResultOutcome::Failed { .. }))
        .count();

    let json_report = serde_json::to_vec_pretty(&rows)
        .change_context(TransformationError::ReportGenerationFailed)
        .attach_printable("Failed to serialize detailed transformation report")?;
    let json_report_file_key = upload_transformation_report(
        state,
        transformation_id,
        transformation_history_id,
        "report-v1.json",
        json_report,
    )
    .await?;

    let csv_report = serialize_readable_transformation_report(rows)?;
    let csv_report_file_key = upload_transformation_report(
        state,
        transformation_id,
        transformation_history_id,
        "report-v1.csv",
        csv_report,
    )
    .await?;

    Ok(TransformationData::V2(TransformationDataV2::Processed {
        csv_report_file_key,
        json_report_file_key,
        success_count,
        failed_count,
        ignored_count,
    }))
}

fn serialize_readable_transformation_report(
    rows: Vec<TransformationRowResult>,
) -> TransformationResult<Vec<u8>> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    if rows.is_empty() {
        writer
            .write_record(["source_type", "identifier", "outcome", "details"])
            .change_context(TransformationError::ReportGenerationFailed)
            .attach_printable("Failed to write readable transformation report header")?;
    } else {
        for row in rows {
            writer
                .serialize(ReadableTransformationReportRow::from(row))
                .change_context(TransformationError::ReportGenerationFailed)
                .attach_printable("Failed to serialize readable transformation report")?;
        }
    }

    writer
        .into_inner()
        .change_context(TransformationError::ReportGenerationFailed)
        .attach_printable("Failed to finish readable transformation report")
}

async fn upload_transformation_report(
    state: &AppState,
    transformation_id: &str,
    transformation_history_id: &str,
    file_name: &str,
    file: Vec<u8>,
) -> TransformationResult<PathBuf> {
    let file_key: PathBuf = [
        "transformations",
        transformation_id,
        transformation_history_id,
        file_name,
    ]
    .iter()
    .collect();
    let file_key_str = file_key
        .to_str()
        .ok_or_else(|| report!(TransformationError::ReportUploadFailed))
        .attach_printable("Failed to construct transformation report key")?;

    state
        .file_storage
        .upload_file(file_key_str, file)
        .await
        .change_context(TransformationError::ReportUploadFailed)
        .attach_printable_lazy(|| format!("Failed to upload transformation report {file_name}"))?;

    Ok(file_key)
}

fn format_report_source(source: &SourceRecord) -> (&'static str, String) {
    match source {
        SourceRecord::FileRow { line_number } => ("file_row", line_number.to_string()),
        SourceRecord::ApiRecord { index } => (
            "api_record",
            index.map_or_else(String::new, |index| index.to_string()),
        ),
    }
}

fn format_matched_conditions(conditions: &[MatchedCondition]) -> String {
    conditions
        .iter()
        .enumerate()
        .map(|(index, condition)| {
            let condition = match condition {
                MatchedCondition::RowSkip {
                    config: RowSkipConfig::LineNo { line_number },
                } => format!("line number equals {line_number}"),
                MatchedCondition::ValueSkip {
                    config,
                    field_value,
                } => format!(
                    "{} {} {}; actual value was {}",
                    config.identifier,
                    format_skip_operator(config.operator),
                    format_optional_value(&config.value),
                    format_optional_value(field_value)
                ),
            };
            format!("{}. {condition}", index + 1)
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_skip_operator(operator: SkipOperator) -> &'static str {
    match operator {
        SkipOperator::Equals => "equals",
        SkipOperator::NotEquals => "does not equal",
        SkipOperator::Contains => "contains",
        SkipOperator::NotContains => "does not contain",
        SkipOperator::StartsWith => "starts with",
        SkipOperator::NotStartsWith => "does not start with",
        SkipOperator::EndsWith => "ends with",
        SkipOperator::NotEndsWith => "does not end with",
    }
}

fn format_optional_value(value: &Option<String>) -> String {
    value
        .as_ref()
        .map_or_else(|| "<missing>".to_string(), |value| format!("\"{value}\""))
}

fn format_failed_details(
    stage: RowFailureStage,
    diagnostics: &[TransformationDiagnostic],
) -> String {
    let stage = match stage {
        RowFailureStage::Parsing => "Parsing",
        RowFailureStage::Validation => "Validation",
        RowFailureStage::Encryption => "Encryption",
    };
    let details = diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            format!(
                "{}. {}: {}",
                index + 1,
                format_source_location(&diagnostic.location),
                diagnostic.error
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    if details.is_empty() {
        format!("{stage} failed")
    } else {
        format!("{stage} failed: {details}")
    }
}

fn format_source_location(location: &SourceLocation) -> String {
    match location {
        SourceLocation::FileRow { line_number } => format!("line {line_number}"),
        SourceLocation::FileCell {
            line_number,
            column,
        } => format!("line {line_number}, column {column}"),
        SourceLocation::FileRange {
            line_number,
            start_column,
            end_column,
        } => format!("line {line_number}, columns {start_column}-{end_column}"),
        SourceLocation::ApiRecord { index } => index.map_or_else(
            || "API record".to_string(),
            |index| format!("API record {index}"),
        ),
        SourceLocation::Field { field_name } => format!("field {field_name}"),
    }
}

/// Convert ParsingConfigApi to ParsingConfig
impl ToDiesel<DieselParsingConfig> for ParsingConfigApi {
    fn to_diesel(self) -> DieselParsingConfig {
        DieselParsingConfig::V1(match self {
            Self::Csv(csv_config) => ParsingConfigV1::Csv(csv_config.to_diesel()),
            Self::Xlsx(xlsx_config) => ParsingConfigV1::Xlsx(xlsx_config.to_diesel()),
            Self::FixedWidth(fixed_width_config) => {
                ParsingConfigV1::FixedWidth(fixed_width_config.to_diesel())
            }
        })
    }
}

/// Convert CsvParsingConfigApi to CsvParsingConfig
impl ToDiesel<DieselCsvParsingConfig> for CsvParsingConfigApi {
    fn to_diesel(self) -> DieselCsvParsingConfig {
        DieselCsvParsingConfig::V1(CsvParsingConfigV1)
    }
}

/// Convert XlsxParsingConfigApi to XlsxParsingConfig
impl ToDiesel<DieselXlsxParsingConfig> for XlsxParsingConfigApi {
    fn to_diesel(self) -> DieselXlsxParsingConfig {
        match self {
            Self::V1(v1) => DieselXlsxParsingConfig::V1(DieselXlsxParsingConfigV1 {
                header_row_index: v1.header_row_index,
                sheet_selection: v1.sheet_selection.to_diesel(),
            }),
        }
    }
}

/// Convert SheetSelectionApi to SheetSelection
impl ToDiesel<SheetSelection> for SheetSelectionApi {
    fn to_diesel(self) -> SheetSelection {
        match self {
            Self::ByIndex(i) => SheetSelection::ByIndex(i),
            Self::ByName(n) => SheetSelection::ByName(n),
        }
    }
}

/// Convert FixedWidthConfigApi to FixedWidthConfig
impl ToDiesel<DieselFixedWidthConfig> for FixedWidthConfigApi {
    fn to_diesel(self) -> DieselFixedWidthConfig {
        DieselFixedWidthConfig::V1(DieselFixedWidthConfigV1 {
            fields: self.fields.into_iter().map(ToDiesel::to_diesel).collect(),
        })
    }
}

/// Convert FixedWidthFieldApi to FixedWidthField
impl ToDiesel<DieselFixedWidthField> for FixedWidthFieldApi {
    fn to_diesel(self) -> DieselFixedWidthField {
        DieselFixedWidthField {
            header: self.header,
            index: self.index,
            width: self.width,
        }
    }
}

impl ToDiesel<SkipConfig> for SkipConfigApi {
    fn to_diesel(self) -> SkipConfig {
        SkipConfig::V1(SkipConfigV1 {
            conditions: self
                .conditions
                .into_iter()
                .map(ToDiesel::to_diesel)
                .collect(),
        })
    }
}

impl ToDiesel<SkipCondition> for SkipConditionApi {
    fn to_diesel(self) -> SkipCondition {
        match self {
            Self::RowSkip(row_skip_config) => SkipCondition::RowSkip(row_skip_config.to_diesel()),
            Self::ValueSkip(value_skip_config) => {
                SkipCondition::ValueSkip(value_skip_config.to_diesel())
            }
        }
    }
}

impl ToDiesel<RowSkipConfig> for RowSkipConfigApi {
    fn to_diesel(self) -> RowSkipConfig {
        match self {
            Self::LineNo { line_number } => RowSkipConfig::LineNo { line_number },
        }
    }
}

impl ToDiesel<ValueSkipConfig> for ValueSkipConfigApi {
    fn to_diesel(self) -> ValueSkipConfig {
        ValueSkipConfig {
            identifier: self.identifier,
            operator: self.operator.to_diesel(),
            value: self.value,
            transformation_rules: self
                .transformation_rules
                .into_iter()
                .map(ToDiesel::to_diesel)
                .collect(),
        }
    }
}

impl ToDiesel<StringTransformationRule> for StringTransformationRuleApi {
    fn to_diesel(self) -> StringTransformationRule {
        StringTransformationRule::V1(match self {
            Self::DefaultValue { value } => StringTransformationRuleV1::DefaultValue { value },
            Self::ToUpperCase => StringTransformationRuleV1::ToUpperCase,
            Self::ToLowerCase => StringTransformationRuleV1::ToLowerCase,
            Self::StripPrefix { prefix } => StringTransformationRuleV1::StripPrefix { prefix },
            Self::StripSuffix { suffix } => StringTransformationRuleV1::StripSuffix { suffix },
            Self::Trim => StringTransformationRuleV1::Trim,
            Self::JsonExtract { pointer } => StringTransformationRuleV1::JsonExtract { pointer },
            Self::Regex { pattern, group } => StringTransformationRuleV1::Regex { pattern, group },
            Self::ReplaceChar { from, to, mode } => {
                StringTransformationRuleV1::ReplaceChar { from, to, mode }
            }
        })
    }
}

impl ToDiesel<SkipOperator> for SkipOperatorApi {
    fn to_diesel(self) -> SkipOperator {
        match self {
            Self::Equals => SkipOperator::Equals,
            Self::NotEquals => SkipOperator::NotEquals,
            Self::Contains => SkipOperator::Contains,
            Self::NotContains => SkipOperator::NotContains,
            Self::StartsWith => SkipOperator::StartsWith,
            Self::NotStartsWith => SkipOperator::NotStartsWith,
            Self::EndsWith => SkipOperator::EndsWith,
            Self::NotEndsWith => SkipOperator::NotEndsWith,
        }
    }
}

pub fn validate_transformation_config_request(
    request: &CreateTransformationConfigV2Request,
) -> TransformationResult<()> {
    if request
        .config
        .skip_configs
        .iter()
        .any(|skip_config| skip_config.conditions.is_empty())
    {
        return Err(report!(TransformationError::EmptySkipConfigConditionsFound));
    }

    Ok(())
}
