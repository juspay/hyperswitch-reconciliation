use api_models::{
    metadata_schema::MetadataSchemaResponseV2,
    transformation::{
        CreateTransformationConfigV2Request, SkipConfig as SkipConfigApi,
        TransformationConfigListResponse, TransformationConfigResponse,
        TransformationHistoryListResponse, TransformationHistoryResponse,
        TransformationReportFormat, TransformationTriggerRequest,
        UpdateTransformationConfigRequest,
    },
};
use common_enums::TransformationStatus;
use common_types::transformation::{
    ListTransformationConfigsFilters, ListTransformationHistoryFilters, TransformationConfigData,
    TransformationConfigDataV2, TransformationFailureStage,
};
use common_utils::auth::AuthContext;
use diesel_models::{
    metadata_schema::{MetadataSchemaNew, MetadataSchemaUpdate},
    transformation_config::{TransformationConfigNew, TransformationConfigUpdate},
    transformation_history::{TransformationHistoryNew, TransformationHistoryUpdate},
};
use error_stack::{report, ResultExt};
use futures::future;

use crate::{
    db::{
        account::AccountInterface,
        ingestion_config::IngestionConfigInterface,
        metadata_schema::MetadataSchemaInterface,
        transformation::{TransformationConfigInterface, TransformationHistoryInterface},
    },
    domain::transformation::TransformationApplicationExecutor,
    errors::{
        conversions::StorageErrorExt,
        transformation::{TransformationError, TransformationResponse},
    },
    routes::AppState,
    types::ApplicationResponse,
    utils::{
        metadata_schema::{MetadataSchemaDataApiExt, MetadataSchemaDataDieselExt},
        transformation::{
            generate_and_upload_transformation_reports, update_transformation_history_failed,
            validate_transformation_config_request, ToDiesel, TransformationConfigExt,
            TransformationConfigHistoryViewExt, TransformationDataApiExt,
        },
    },
};

pub async fn execute_transformation(
    state: AppState,
    ingestion_id: String,
    ingestion_history_id: String,
    request: TransformationTriggerRequest,
) -> TransformationResponse<()> {
    let transformation_configs = state
        .store
        .list_transformation_configs_by_ingestion_id(&ingestion_id, Some(true))
        .await
        .change_context(TransformationError::InternalServerError)?;

    if transformation_configs.is_empty() {
        return Err(TransformationError::ConfigNotFound { ingestion_id }.into());
    }

    let configs_with_latest_history =
        future::try_join_all(transformation_configs.into_iter().map(async |config| {
            let transformation_history_new = TransformationHistoryNew::new(
                config.id.clone(),
                ingestion_history_id.clone(),
                config.profile_id.clone(),
                config.account_id.clone(),
                TransformationStatus::Pending,
                None,
            );

            state
                .store
                .insert_transformation_history(transformation_history_new)
                .await
                .map(|history| (config, history))
                .change_context(TransformationError::InternalServerError)
        }))
        .await?;

    let file_data = match state
        .file_storage
        .retrieve_file(
            request
                .file_path
                .to_str()
                .ok_or(TransformationError::InvalidFileName)?,
        )
        .await
        .change_context(TransformationError::InternalServerError)
    {
        Ok(file) => file,
        Err(e) => {
            future::try_join_all(configs_with_latest_history.iter().map(
                async |(_config, history)| {
                    update_transformation_history_failed(
                        &state,
                        history.id.as_str(),
                        TransformationFailureStage::FileRetrieval,
                        &format!("File retrieval failed: {e}"),
                    )
                    .await
                },
            ))
            .await?;

            return Err(e);
        }
    };

    for (config, history) in configs_with_latest_history {
        let history = state
            .store
            .update_transformation_history_by_id(
                &history.id,
                TransformationHistoryUpdate::Processing,
            )
            .await
            .change_context(TransformationError::InternalServerError)?;

        // Execute transformation with error handling
        let transformation_result = match config
            .config
            .transform_file(
                &file_data,
                &config.id,
                &history.id,
                &config.metadata_schema_id,
                &state,
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                let failure_stage = match e.current_context() {
                    TransformationError::InvalidFileFormat { .. } => {
                        TransformationFailureStage::FileParsing
                    }
                    _ => TransformationFailureStage::TransformationExecution,
                };
                let _ = update_transformation_history_failed(
                    &state,
                    &history.id,
                    failure_stage,
                    &format!("Transformation execution failed: {e}"),
                )
                .await;
                return Err(e);
            }
        };

        let transformation_data = match generate_and_upload_transformation_reports(
            &state,
            &config.id,
            &history.id,
            transformation_result.rows,
        )
        .await
        {
            Ok(transformation_data) => transformation_data,
            Err(e) => {
                let _ = update_transformation_history_failed(
                    &state,
                    &history.id,
                    TransformationFailureStage::TransformationReportUpload,
                    &format!("{e:#}"),
                )
                .await;
                return Err(e);
            }
        };

        // Insert staging entries with error handling
        if let Err(e) = state
            .store
            .batch_insert_staging_entries(transformation_result.staging_entries_new)
            .await
            .change_context(TransformationError::InternalServerError)
        {
            let _ = update_transformation_history_failed(
                &state,
                &history.id,
                TransformationFailureStage::StagingEntryInsertion,
                &format!("Failed to insert staging entries: {e}"),
            )
            .await;
            return Err(e);
        }

        // Update history to success
        state
            .store
            .update_transformation_history_by_id(
                &history.id,
                TransformationHistoryUpdate::Processed {
                    processed_data: transformation_data,
                },
            )
            .await
            .change_context(TransformationError::InternalServerError)?;
    }

    Ok(ApplicationResponse::StatusOk)
}

pub async fn create_transformation_config_v2(
    state: AppState,
    request: CreateTransformationConfigV2Request,
    auth_context: AuthContext,
) -> TransformationResponse<TransformationConfigResponse> {
    // Validate request data
    validate_transformation_config_request(&request)?;

    // Validate ingestion_id exists and belongs to the profile_id from auth
    state
        .store
        .find_ingestion_config_by_profile_id_and_ingestion_id(
            &auth_context.profile_id,
            &request.ingestion_id,
        )
        .await
        .to_not_found_response(|| TransformationError::IngestionConfigNotFound {
            ingestion_id: request.ingestion_id.clone(),
        })?;

    // Validate account_id exists and belongs to the profile_id from auth
    state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &request.account_id)
        .await
        .to_not_found_response(|| TransformationError::AccountNotFound {
            account_id: request.account_id.clone(),
        })?;

    // Convert API metadata schema data to database format
    let diesel_schema_data = request.metadata_schema_data.to_diesel_schema();

    // Create metadata schema first
    let metadata_schema_new = MetadataSchemaNew::new(
        auth_context.profile_id.clone(),
        request.account_id.clone(),
        diesel_schema_data,
    );

    // Create V2 transformation config with the generated metadata schema ID
    let transformation_config = TransformationConfigDataV2 {
        merchant_id: auth_context.merchant_id.clone(),
        profile_id: auth_context.profile_id.clone(),
        account_id: request.account_id.clone(),
        parsing_config: request.config.parsing_config.to_diesel(),
        skip_configs: request
            .config
            .skip_configs
            .into_iter()
            .map(ToDiesel::to_diesel)
            .collect(),
    };

    let transformation_config_new = TransformationConfigNew::new(
        auth_context.profile_id,
        request.ingestion_id,
        request.account_id,
        request.name,
        TransformationConfigData::V2(transformation_config),
        request.is_active,
        metadata_schema_new.id.clone(),
    );

    // Insert both in a transaction
    let config = state
        .store
        .insert_transformation_config_with_metadata_schema(
            transformation_config_new,
            metadata_schema_new,
        )
        .await
        .change_context(TransformationError::InternalServerError)?;

    Ok(ApplicationResponse::json_created(config.to_response(None)))
}

pub async fn list_transformation_configs_by_profile_id(
    state: AppState,
    filters: ListTransformationConfigsFilters,
    auth_context: AuthContext,
) -> TransformationResponse<TransformationConfigListResponse> {
    // Validate filters
    filters
        .validate()
        .map_err(|_| TransformationError::TimeRangeValidationError)?;

    let configs = state
        .store
        .list_transformation_configs_by_profile_id(&auth_context.profile_id, filters)
        .await
        .change_context(TransformationError::InternalServerError)?;

    let configs_with_latest_history =
        future::try_join_all(configs.into_iter().map(async |config| {
            let history = state
                .store
                .list_transformation_history_by_transformation_id(&config.id, Some(1))
                .await
                .change_context(TransformationError::InternalServerError)?
                .pop();

            Ok::<_, error_stack::Report<TransformationError>>((config, history))
        }))
        .await?;

    let config_responses: Vec<TransformationConfigResponse> = configs_with_latest_history
        .into_iter()
        .map(|(config, history)| {
            let last_transformed_at = history.map(|h| h.created_at);
            config.to_response(last_transformed_at)
        })
        .collect();

    Ok(ApplicationResponse::json(config_responses))
}

pub async fn find_transformation_history_with_config_by_id(
    state: &AppState,
    transformation_history_id: &str,
    auth_profile_id: &str,
) -> TransformationResponse<TransformationHistoryResponse> {
    let data = state
        .store
        .find_config_and_history_by_id_and_profile(transformation_history_id, auth_profile_id)
        .await
        .to_not_found_response(|| TransformationError::TransformationHistoryNotFound {
            id: transformation_history_id.to_string(),
        })?;

    let api_response = data.to_api_response();

    Ok(ApplicationResponse::json(api_response))
}

pub async fn download_transformation_report(
    state: &AppState,
    transformation_history_id: &str,
    report_format: TransformationReportFormat,
    auth_profile_id: &str,
) -> TransformationResponse<()> {
    let view = state
        .store
        .find_config_and_history_by_id_and_profile(transformation_history_id, auth_profile_id)
        .await
        .to_not_found_response(|| TransformationError::TransformationHistoryNotFound {
            id: transformation_history_id.to_string(),
        })?;

    let report_file_key = view
        .history
        .data
        .and_then(|data| data.get_report_file_key(report_format))
        .ok_or_else(|| report!(TransformationError::ReportNotAvailable))?;

    let report_file_key = report_file_key
        .to_str()
        .ok_or_else(|| report!(TransformationError::InternalServerError))?;
    let report = state
        .file_storage
        .retrieve_file(report_file_key)
        .await
        .change_context(TransformationError::InternalServerError)?;

    let (extension, content_type) = match report_format {
        TransformationReportFormat::Csv => ("csv", mime::TEXT_CSV),
        TransformationReportFormat::Json => ("json", mime::APPLICATION_JSON),
    };

    Ok(ApplicationResponse::FileDownload {
        data: report,
        filename: format!("transformation_report_{transformation_history_id}.{extension}"),
        content_type,
    })
}

pub async fn list_transformation_history_with_config_by_profile_id(
    state: &AppState,
    auth_profile_id: String,
    filters: ListTransformationHistoryFilters,
) -> TransformationResponse<TransformationHistoryListResponse> {
    let data = state
        .store
        .list_history_with_config_by_profile_id(&auth_profile_id, filters)
        .await
        .change_context(TransformationError::InternalServerError)?;

    let api_response = data
        .into_iter()
        .map(|view| view.to_api_response())
        .collect::<Vec<_>>();

    Ok(ApplicationResponse::json(api_response))
}

pub async fn get_transformation_config_by_id(
    state: AppState,
    transformation_id: String,
    auth_context: AuthContext,
) -> TransformationResponse<TransformationConfigResponse> {
    let config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &transformation_id,
            &auth_context.profile_id,
        )
        .await
        .to_not_found_response(|| TransformationError::TransformationConfigNotFound {
            id: transformation_id,
        })?;

    // Get the latest transformation history for this config
    let history = state
        .store
        .list_transformation_history_by_transformation_id(&config.id, Some(1))
        .await
        .change_context(TransformationError::InternalServerError)?
        .pop();

    let last_transformed_at = history.map(|h| h.created_at);

    Ok(ApplicationResponse::json(
        config.to_response(last_transformed_at),
    ))
}

pub async fn update_transformation_config(
    state: AppState,
    transformation_id: String,
    request: UpdateTransformationConfigRequest,
    auth_context: AuthContext,
) -> TransformationResponse<TransformationConfigResponse> {
    if let Some(ref name) = request.name {
        if name.trim().is_empty() {
            return Err(TransformationError::InvalidName.into());
        }
    }

    if let Some(ref skip_configs) = request.skip_configs {
        if skip_configs
            .iter()
            .any(|skip_config| skip_config.conditions.is_empty())
        {
            return Err(report!(TransformationError::EmptySkipConfigConditionsFound));
        }
    }

    let existing_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &transformation_id,
            &auth_context.profile_id,
        )
        .await
        .to_not_found_response(|| TransformationError::TransformationConfigNotFound {
            id: transformation_id.clone(),
        })?;

    let updated_config = if let Some(name) = request.name {
        state
            .store
            .update_transformation_config_by_id(
                &transformation_id,
                TransformationConfigUpdate::NameUpdate { name },
            )
            .await
            .change_context(TransformationError::InternalServerError)?
    } else {
        existing_config
    };

    let updated_config = if request.skip_configs.is_some() || request.parsing_config.is_some() {
        let new_config_data = match updated_config.config {
            TransformationConfigData::V2(mut config_v2) => {
                if let Some(parsing_config) = request.parsing_config {
                    config_v2.parsing_config = parsing_config.to_diesel();
                };

                if let Some(skip_configs) = request.skip_configs {
                    config_v2.skip_configs = skip_configs
                        .into_iter()
                        .map(SkipConfigApi::to_diesel)
                        .collect();
                };

                TransformationConfigData::V2(config_v2)
            }
        };

        state
            .store
            .update_transformation_config_by_id(
                &transformation_id,
                TransformationConfigUpdate::ConfigUpdate {
                    config: new_config_data,
                },
            )
            .await
            .change_context(TransformationError::InternalServerError)?
    } else {
        updated_config
    };

    if let Some(metadata_schema_data) = request.metadata_schema_data {
        let diesel_schema_data = metadata_schema_data.to_diesel_schema();
        state
            .store
            .update_metadata_schema_by_id(
                &updated_config.metadata_schema_id,
                MetadataSchemaUpdate::SchemaUpdate {
                    schema_data: diesel_schema_data,
                },
            )
            .await
            .change_context(TransformationError::InternalServerError)?;
    }

    let updated_config = if let Some(is_active) = request.is_active {
        state
            .store
            .update_transformation_config_by_id(
                &transformation_id,
                TransformationConfigUpdate::ActiveStatusUpdate { is_active },
            )
            .await
            .change_context(TransformationError::InternalServerError)?
    } else {
        updated_config
    };

    // Get the latest transformation history for this config
    let history = state
        .store
        .list_transformation_history_by_transformation_id(&updated_config.id, Some(1))
        .await
        .change_context(TransformationError::InternalServerError)?
        .pop();

    let last_transformed_at = history.map(|h| h.created_at);

    Ok(ApplicationResponse::json(
        updated_config.to_response(last_transformed_at),
    ))
}

pub async fn get_metadata_schema_by_transformation_id(
    state: AppState,
    transformation_id: String,
    auth_context: AuthContext,
) -> TransformationResponse<MetadataSchemaResponseV2> {
    let transformation_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &transformation_id,
            &auth_context.profile_id,
        )
        .await
        .to_not_found_response(|| TransformationError::TransformationConfigNotFound {
            id: transformation_id.clone(),
        })?;

    let metadata_schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(
            &transformation_config.metadata_schema_id,
            &auth_context.profile_id,
        )
        .await
        .to_not_found_response(
            || TransformationError::MetadataSchemaNotFoundForTransformation { transformation_id },
        )?;

    Ok(ApplicationResponse::json(metadata_schema.to_api_response()))
}
