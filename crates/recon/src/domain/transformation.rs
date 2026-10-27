use common_enums::StagingEntryProcessingMode;
use common_types::{
    metadata_schema::ProcessingModeProvider,
    transformation::{TransformationConfigData, TransformationConfigDataV2},
};
use common_utils::{encryption::Encryption, id_type::MerchantId};
use diesel_models::staging_entry::StagingEntryNew;
use error_stack::ResultExt;
use rustc_hash::FxHashMap;
use storage_impl::metadata_schema::MetadataSchemaInterface;

pub mod types;

use crate::{
    domain::{
        metadata_schema::{
            data_validator::RowValidator,
            mapper::RowMapper,
            skip_config::{evaluate_skip, SkipDecision},
            types::{
                FileParsingOutcome, RowParsingOutcome, RowValidationOutcome, ValidatedEntryData,
            },
        },
        parsing_engine::FileParser,
        transformation::types::{
            RowFailureStage, RowOutcome, RowResultOutcome, SourceLocation, SourceRecord, StagedRow,
            StagingEntryContext, TransformationDiagnostic, TransformationExecutionResult,
            TransformationInput, TransformationOutput, TransformationRowOutput,
            TransformationRowResult, TransformationStagingEntryData,
        },
    },
    errors::{
        conversions::StorageErrorExt,
        transformation::{TransformationError, TransformationResult},
    },
    routes::AppState,
};

pub trait TransformationEngine {
    fn transform_input(&self, input: TransformationInput<'_>) -> TransformationOutput;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultTransformationEngine;

impl TransformationEngine for DefaultTransformationEngine {
    fn transform_input(&self, input: TransformationInput<'_>) -> TransformationOutput {
        let parsed_rows = match input.config.parsing_config.parse(input.file) {
            FileParsingOutcome::Parsed { rows } => rows,
            FileParsingOutcome::Rejected { failure } => {
                return TransformationOutput::Rejected { failure }
            }
        };

        let rows = parsed_rows
            .into_iter()
            .map(|parsing_outcome| match parsing_outcome {
                RowParsingOutcome::Rejected(error) => {
                    let source = source_record_from_location(&error.location);
                    TransformationRowOutput {
                        source,
                        outcome: RowOutcome::Failed {
                            stage: RowFailureStage::Parsing,
                            errors: vec![TransformationDiagnostic {
                                location: error.location,
                                error: error.message,
                            }],
                        },
                    }
                }
                RowParsingOutcome::Parsed(row) => {
                    let source = row.source.clone();
                    match evaluate_skip(&input.config.skip_configs, &row) {
                        SkipDecision::Skip(reason) => TransformationRowOutput {
                            source,
                            outcome: RowOutcome::Skipped { reason },
                        },
                        SkipDecision::Keep => {
                            let mapped = input.schema.map_row(row);
                            match input.schema.validate_row(&mapped) {
                                RowValidationOutcome::Validated(validated) => {
                                    TransformationRowOutput {
                                        source,
                                        outcome: RowOutcome::Success(validated),
                                    }
                                }
                                RowValidationOutcome::Rejected(rejection) => {
                                    TransformationRowOutput {
                                        source,
                                        outcome: RowOutcome::Failed {
                                            stage: RowFailureStage::Validation,
                                            errors: rejection
                                                .field_errors
                                                .into_iter()
                                                .map(|located| TransformationDiagnostic {
                                                    location: located.location,
                                                    error: located.value.to_string(),
                                                })
                                                .collect(),
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .collect();

        TransformationOutput::Processed { rows }
    }
}

fn source_record_from_location(location: &SourceLocation) -> SourceRecord {
    match location {
        SourceLocation::FileRow { line_number }
        | SourceLocation::FileCell { line_number, .. }
        | SourceLocation::FileRange { line_number, .. } => SourceRecord::FileRow {
            line_number: *line_number,
        },
        SourceLocation::ApiRecord { index } => SourceRecord::ApiRecord { index: *index },
        SourceLocation::Field { .. } => SourceRecord::ApiRecord { index: None },
    }
}

/// Temporary application adapter around the pure transformation engine.
pub trait TransformationApplicationExecutor {
    async fn transform_file(
        &self,
        file_data: &[u8],
        transformation_id: &str,
        transformation_history_id: &str,
        metadata_schema_id: &str,
        state: &AppState,
    ) -> TransformationResult<TransformationExecutionResult>;
}

impl TransformationApplicationExecutor for TransformationConfigData {
    async fn transform_file(
        &self,
        file_data: &[u8],
        transformation_id: &str,
        transformation_history_id: &str,
        metadata_schema_id: &str,
        state: &AppState,
    ) -> TransformationResult<TransformationExecutionResult> {
        match self {
            Self::V2(config_v2) => {
                config_v2
                    .transform_file(
                        file_data,
                        transformation_id,
                        transformation_history_id,
                        metadata_schema_id,
                        state,
                    )
                    .await
            }
        }
    }
}

impl TransformationApplicationExecutor for TransformationConfigDataV2 {
    async fn transform_file(
        &self,
        file_data: &[u8],
        transformation_id: &str,
        transformation_history_id: &str,
        metadata_schema_id: &str,
        state: &AppState,
    ) -> TransformationResult<TransformationExecutionResult> {
        let schema = state
            .store
            .find_metadata_schema_by_id_and_profile_id(metadata_schema_id, &self.profile_id)
            .await
            .to_not_found_response(|| TransformationError::MetadataSchemaNotFound {
                metadata_schema_id: metadata_schema_id.to_string(),
            })?;

        let transformation_output =
            DefaultTransformationEngine.transform_input(TransformationInput {
                file: file_data,
                config: self,
                schema: &schema.schema_data,
            });

        let engine_rows = match transformation_output {
            TransformationOutput::Processed { rows } => rows,
            TransformationOutput::Rejected { failure } => {
                Err(TransformationError::InvalidFileFormat {
                    details: failure.to_string(),
                })?
            }
        };

        let processing_mode = schema.schema_data.get_processing_mode();

        let staged_rows = engine_rows
            .into_iter()
            .map(|row| match row.outcome {
                RowOutcome::Success(validated) => StagedRow::AwaitingEncryption {
                    entry_data: build_staging_entry_data(validated, row.source, processing_mode),
                },
                RowOutcome::Skipped { reason } => StagedRow::Completed {
                    result: TransformationRowResult {
                        source: row.source,
                        outcome: RowResultOutcome::Skipped { reason },
                    },
                },
                RowOutcome::Failed { stage, errors } => StagedRow::Completed {
                    result: TransformationRowResult {
                        source: row.source,
                        outcome: RowResultOutcome::Failed { stage, errors },
                    },
                },
            })
            .collect::<Vec<_>>();

        let encrypted_metadata_batch =
            encrypt_row_metadata(&staged_rows, &self.merchant_id, &state.encryption_client).await?;

        let context = StagingEntryContext {
            account_id: &self.account_id,
            merchant_id: &self.merchant_id,
            profile_id: &self.profile_id,
            transformation_id,
            transformation_history_id,
        };

        Ok(staged_rows
            .into_iter()
            .map(|staged_row| resolve_staged_row(staged_row, &encrypted_metadata_batch, &context))
            .collect())
    }
}

fn build_staging_entry_data(
    validated: ValidatedEntryData,
    source: SourceRecord,
    processing_mode: StagingEntryProcessingMode,
) -> TransformationStagingEntryData {
    TransformationStagingEntryData::new(
        source,
        validated.row_data.balance_direction,
        validated.row_data.amount,
        validated.row_data.currency,
        validated.row_data.effective_at,
        processing_mode,
        validated.row_data.order_id,
        Some(validated.row_data.metadata),
        validated.row_data.exposed_metadata,
        validated.row_data.searchable_metadata,
    )
}

async fn encrypt_row_metadata(
    staged_rows: &[StagedRow],
    merchant_id: &MerchantId,
    encryption_client: &external_services::ApplicationEncryptionClient,
) -> TransformationResult<FxHashMap<String, Option<Encryption>>> {
    let uuid_metadata: FxHashMap<String, Option<serde_json::Value>> = staged_rows
        .iter()
        .filter_map(|staged_row| match staged_row {
            StagedRow::AwaitingEncryption { entry_data } => {
                Some((entry_data.uuid.to_string(), entry_data.metadata.clone()))
            }
            StagedRow::Completed { .. } => None,
        })
        .collect();

    encryption_client
        .get_encryptor_for_merchant(merchant_id.clone())
        .encrypt_batch_optional_json_values(uuid_metadata)
        .await
        .change_context(TransformationError::InternalServerError)
}

fn resolve_staged_row(
    staged_row: StagedRow,
    encrypted_metadata_batch: &FxHashMap<String, Option<Encryption>>,
    context: &StagingEntryContext<'_>,
) -> (TransformationRowResult, Option<StagingEntryNew>) {
    let entry_data = match staged_row {
        StagedRow::Completed { result } => return (result, None),
        StagedRow::AwaitingEncryption { entry_data } => entry_data,
    };

    let uuid = entry_data.uuid.to_string();
    let source = entry_data.source_record.clone();

    let staging_entry = encrypted_metadata_batch
        .get(&uuid)
        .and_then(|metadata| metadata.as_ref().cloned())
        .map(|metadata| build_staging_entry(entry_data, metadata, context));

    let outcome = match &staging_entry {
        Some(entry) => RowResultOutcome::Transformed {
            staging_entry_id: entry.staging_entry_id.clone(),
        },
        None => RowResultOutcome::Failed {
            stage: RowFailureStage::Encryption,
            errors: vec![TransformationDiagnostic {
                location: source.clone().into(),
                error: "Failed to find encrypted metadata for entry".to_string(),
            }],
        },
    };

    (TransformationRowResult { source, outcome }, staging_entry)
}

fn build_staging_entry(
    entry_data: TransformationStagingEntryData,
    metadata: Encryption,
    context: &StagingEntryContext<'_>,
) -> StagingEntryNew {
    StagingEntryNew::new(
        context.account_id.to_string(),
        context.merchant_id.clone(),
        entry_data.row_data.balance_direction,
        entry_data.row_data.amount,
        entry_data.row_data.currency,
        entry_data.row_data.effective_at,
        metadata,
        entry_data.row_data.processing_mode,
        context.profile_id.to_string(),
        context.transformation_id.to_string(),
        Some(context.transformation_history_id.to_string()),
        entry_data.row_data.order_id,
        None,
        entry_data.exposed_metadata,
        entry_data.searchable_metadata,
    )
}
