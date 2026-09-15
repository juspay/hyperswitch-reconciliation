use crate::{
    db::{
        account::AccountInterface, business_profile::BusinessProfileInterface,
        ingestion_config::IngestionConfigInterface, ingestion_history::IngestionHistoryInterface,
    },
    domain::ingestion::{IngestionConfigDataExt, IngestionContext},
    errors::{
        conversions::StorageErrorExt,
        ingestion::{
            IngestionError, IngestionResponse, IngestionResult, ParseStageError, StorageStageError,
        },
    },
    routes::AppState,
    types::ApplicationResponse,
    utils::ingestion::{self as utils, IngestionConfigHistoryViewExt, IngestionDataExt},
};
use actix_web::http::header::HeaderMap;
use api_models::ingestion::{
    IngestionConfigCreateRequest, IngestionConfigListResponse, IngestionConfigResponse,
    IngestionConfigUpdateRequest, IngestionHistoryListResponse, IngestionHistoryResponse,
    ProcessedIngestionFileRequest,
};
use common_enums::IngestionStatus;
use common_types::ingestion::{
    IngestionData, IngestionDataV1, IngestionFailureData, IngestionFailureStage,
    IngestionProcessedData, ListIngestionConfigsFilters, ListIngestionHistoryFilters,
};
use common_utils::{date_time, errors::ReportSwitchExt};
use diesel_models::{
    ingestion_config::{IngestionConfigNew, IngestionConfigUpdate},
    ingestion_history::{IngestionHistoryNew, IngestionHistoryVersionUpdate},
};
use error_stack::ResultExt;
use futures::future;
use recon_domain_models::{
    behaviour::ReverseConversion,
    ingestion::{to_diesel_config, DomainIngestionConfig},
};
use serde_json::Value;

pub async fn create_ingestion_config(
    state: &AppState,
    request: IngestionConfigCreateRequest,
) -> IngestionResponse<IngestionConfigResponse> {
    let merchant_id = request.merchant_id.clone();

    // Validate profile_id exists
    state
        .store
        .find_business_profile_by_id(&request.profile_id)
        .await
        .change_context(IngestionError::ProfileNotFound {
            profile_id: request.profile_id.clone(),
        })?;

    // Validate account_id exists and belongs to the profile_id
    state
        .store
        .find_account_by_profile_id_and_account_id(&request.profile_id, &request.account_id)
        .await
        .change_context(IngestionError::AccountNotFound {
            account_id: request.account_id.clone(),
        })?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id.clone());

    // Convert API request to Diesel model
    let diesel_config = to_diesel_config(request.config, &merchant_encryptor)
        .await
        .change_context(IngestionError::InternalServerError)?;

    let ingestion_config_new = IngestionConfigNew::new(
        request.profile_id,
        request.account_id,
        request.name,
        request.description,
        diesel_config,
        request.is_active,
        merchant_id,
    );

    let diesel_ingestion_config = state
        .store
        .insert_ingestion_config(ingestion_config_new)
        .await
        .change_context(IngestionError::InternalServerError)?;

    // Convert Diesel model to domain model with decryption
    let domain_config: DomainIngestionConfig = diesel_ingestion_config
        .convert_back(&merchant_encryptor)
        .await
        .change_context(IngestionError::InternalServerError)?;

    Ok(ApplicationResponse::json_created(IngestionConfigResponse {
        ingestion_id: domain_config.ingestion_id,
        account_id: domain_config.account_id,
        is_active: domain_config.is_active,
        name: domain_config.name,
        created_at: domain_config.created_at,
        last_synced_at: None,
        data: domain_config.config.to_api_connection_data(),
    }))
}

pub async fn list_ingestion_configs(
    state: AppState,
    auth_profile_id: &str,
    filters: ListIngestionConfigsFilters,
) -> IngestionResponse<IngestionConfigListResponse> {
    if filters.validate().is_err() {
        return Err(ParseStageError::TimeRangeValidation.into()).switch();
    }

    let configs = state
        .store
        .list_ingestion_configs_by_profile_id(auth_profile_id, filters)
        .await
        .change_context(IngestionError::InternalServerError)?;

    let configs_with_latest_history =
        future::try_join_all(configs.into_iter().map(async |config| {
            let history = state
                .store
                .list_ingestion_history_by_ingestion_id(&config.id, Some(1))
                .await
                .change_context(IngestionError::InternalServerError)?
                .pop();

            IngestionResult::Ok((config, history))
        }))
        .await?;

    let api_response = future::try_join_all(configs_with_latest_history.into_iter().map(
        async |(config, history)| {
            let merchant_encryptor = state
                .encryption_client
                .get_encryptor_for_merchant(config.merchant_id.clone());

            // Diesel → Domain (with decryption)
            let domain_config: DomainIngestionConfig = config
                .convert_back(&merchant_encryptor)
                .await
                .change_context(IngestionError::InternalServerError)?;

            // Domain → API
            IngestionResult::Ok(IngestionConfigResponse {
                ingestion_id: domain_config.ingestion_id,
                account_id: domain_config.account_id,
                is_active: domain_config.is_active,
                name: domain_config.name,
                created_at: domain_config.created_at,
                last_synced_at: history.map(|history| history.created_at),
                data: domain_config.config.to_api_connection_data(),
            })
        },
    ))
    .await?;

    Ok(ApplicationResponse::json(api_response))
}

pub async fn get_ingestion_config(
    state: &AppState,
    auth_profile_id: &str,
    ingestion_id: String,
) -> IngestionResponse<IngestionConfigResponse> {
    let config = state
        .store
        .find_ingestion_config_by_profile_id_and_ingestion_id(auth_profile_id, &ingestion_id)
        .await
        .to_not_found_response(|| IngestionError::IngestionConfigNotFound {
            ingestion_id: ingestion_id.clone(),
        })?;

    let history = state
        .store
        .list_ingestion_history_by_ingestion_id(&ingestion_id, Some(1))
        .await
        .change_context(IngestionError::InternalServerError)?
        .pop();

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(config.merchant_id.clone());

    // Convert Diesel model to domain model with decryption
    let domain_config: DomainIngestionConfig = config
        .convert_back(&merchant_encryptor)
        .await
        .change_context(IngestionError::InternalServerError)?;

    Ok(ApplicationResponse::json(IngestionConfigResponse {
        ingestion_id: domain_config.ingestion_id,
        account_id: domain_config.account_id,
        is_active: domain_config.is_active,
        name: domain_config.name,
        created_at: domain_config.created_at,
        last_synced_at: history.map(|history| history.created_at),
        data: domain_config.config.to_api_connection_data(),
    }))
}

pub async fn update_ingestion_config(
    state: &AppState,
    ingestion_id: String,
    request: IngestionConfigUpdateRequest,
) -> IngestionResponse<IngestionConfigResponse> {
    let existing_config = state
        .store
        .find_ingestion_config_by_id(&ingestion_id)
        .await
        .to_not_found_response(|| IngestionError::IngestionConfigNotFound {
            ingestion_id: ingestion_id.clone(),
        })?;

    let updated_config = if let Some(is_active) = request.is_active {
        let update = IngestionConfigUpdate::UpdateStatus { is_active };
        state
            .store
            .update_ingestion_config_by_id(&ingestion_id, update)
            .await
            .change_context(IngestionError::InternalServerError)?
    } else {
        existing_config
    };

    let updated_config = if let Some(config_data) = request.config {
        let merchant_id = updated_config.merchant_id.clone();

        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(merchant_id);

        // Convert API request to Diesel model
        let diesel_config = to_diesel_config(config_data, &encryptor)
            .await
            .change_context(IngestionError::InternalServerError)?;

        let update = IngestionConfigUpdate::UpdateConfig {
            config: diesel_config,
        };

        state
            .store
            .update_ingestion_config_by_id(&ingestion_id, update)
            .await
            .change_context(IngestionError::InternalServerError)?
    } else {
        updated_config
    };

    let history = state
        .store
        .list_ingestion_history_by_ingestion_id(&ingestion_id, Some(1))
        .await
        .change_context(IngestionError::InternalServerError)?
        .pop();

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(updated_config.merchant_id.clone());

    let domain_config: DomainIngestionConfig = updated_config
        .convert_back(&merchant_encryptor)
        .await
        .change_context(IngestionError::InternalServerError)?;

    Ok(ApplicationResponse::json(IngestionConfigResponse {
        ingestion_id: domain_config.ingestion_id,
        account_id: domain_config.account_id,
        is_active: domain_config.is_active,
        name: domain_config.name,
        created_at: domain_config.created_at,
        last_synced_at: history.map(|history| history.created_at),
        data: domain_config.config.to_api_connection_data(),
    }))
}

pub async fn find_ingestion_history_with_config_by_id(
    state: &AppState,
    id: &str,
) -> IngestionResponse<IngestionHistoryResponse> {
    let data = state
        .store
        .find_config_and_history_by_id(id)
        .await
        .to_not_found_response(|| IngestionError::IngestionHistoryNotFound {
            history_id: id.to_owned(),
        })?;

    let api_response = data.to_api_response();

    Ok(ApplicationResponse::json(api_response))
}

pub async fn list_ingestion_history(
    state: &AppState,
    auth_profile_id: String,
    filters: ListIngestionHistoryFilters,
) -> IngestionResponse<IngestionHistoryListResponse> {
    let data = state
        .store
        .list_history_with_config_by_profile_id(&auth_profile_id, filters)
        .await
        .change_context(IngestionError::InternalServerError)?;

    let api_response = data
        .into_iter()
        .map(|view| view.to_api_response())
        .collect::<Vec<_>>();

    Ok(ApplicationResponse::json(api_response))
}

/// Main entry point for all ingestion triggers
pub async fn trigger_ingestion(
    state: &AppState,
    ingestion_id: String,
    raw_payload: Value,
    headers: &HeaderMap,
) -> IngestionResponse<()> {
    let ingestion_config = state
        .store
        .find_ingestion_config_by_id(&ingestion_id)
        .await
        .to_not_found_response(|| IngestionError::IngestionConfigNotFound {
            ingestion_id: ingestion_id.clone(),
        })?;

    if !ingestion_config.is_active {
        return Err(error_stack::report!(IngestionError::IngestionNotActive));
    }

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(ingestion_config.merchant_id.clone());

    let context = ingestion_config
        .config
        .create_context_from_webhook(raw_payload, headers.clone(), &merchant_encryptor)
        .await
        .change_context(IngestionError::InternalServerError)?;

    process_ingestion(
        state,
        ingestion_id,
        ingestion_config.profile_id,
        ingestion_config.account_id,
        context,
    )
    .await?;

    Ok(ApplicationResponse::StatusOk)
}

/// Core processing logic for both trigger and manual upload
async fn process_ingestion(
    state: &AppState,
    ingestion_id: String,
    profile_id: String,
    account_id: String,
    context: IngestionContext,
) -> IngestionResult<()> {
    let typed_raw_payload = context.get_raw_payload();

    let initial_history = IngestionHistoryNew::new(
        ingestion_id.clone(),
        profile_id.clone(),
        account_id.clone(),
        IngestionStatus::Pending,
        None,
        None,
        typed_raw_payload,
    );

    let initial_inserted = state
        .store
        .insert_ingestion_history(initial_history)
        .await
        .change_context(IngestionError::InternalServerError)?;

    let history_id = initial_inserted.ingestion_history_id.clone();

    let trigger_data = match context.parse_payload().await {
        Ok(data) => data,
        Err(e) => {
            let failure_data = IngestionData::V1(IngestionDataV1::Failed(IngestionFailureData {
                stage: IngestionFailureStage::Parse,
                error_message: format!("Failed to parse webhook data: {e}"),
            }));

            let versioned_update =
                IngestionHistoryVersionUpdate::FailedUpdate { data: failure_data };

            state
                .store
                .create_ingestion_history_version_and_discard_previous(
                    initial_inserted,
                    versioned_update,
                )
                .await
                .change_context(IngestionError::InternalServerError)?;

            return Err(e).switch();
        }
    };

    // Create Processing version with trigger data and discard previous version
    let processing_update = IngestionHistoryVersionUpdate::ProcessingUpdate {
        trigger_data: trigger_data.clone(),
    };

    let processing_inserted = state
        .store
        .create_ingestion_history_version_and_discard_previous(initial_inserted, processing_update)
        .await
        .change_context(IngestionError::InternalServerError)?;

    let download_context = match context.create_download_context().await {
        Ok(ctx) => ctx,
        Err(e) => {
            let failure_data = IngestionData::V1(IngestionDataV1::Failed(IngestionFailureData {
                stage: IngestionFailureStage::Download,
                error_message: format!("Failed to create download context: {e}"),
            }));

            let versioned_update =
                IngestionHistoryVersionUpdate::FailedUpdate { data: failure_data };

            state
                .store
                .create_ingestion_history_version_and_discard_previous(
                    processing_inserted.clone(),
                    versioned_update,
                )
                .await
                .change_context(IngestionError::InternalServerError)?;

            return Err(e);
        }
    };

    let file_result = match download_context.download_file(state).await {
        Ok(file_data) => file_data,
        Err(e) => {
            let failure_data = IngestionData::V1(IngestionDataV1::Failed(IngestionFailureData {
                stage: IngestionFailureStage::Download,
                error_message: format!("Failed to download file: {e}"),
            }));

            let versioned_update =
                IngestionHistoryVersionUpdate::FailedUpdate { data: failure_data };

            state
                .store
                .create_ingestion_history_version_and_discard_previous(
                    processing_inserted.clone(),
                    versioned_update,
                )
                .await
                .change_context(IngestionError::InternalServerError)?;

            return Err(e).switch();
        }
    };

    let file_key = utils::generate_file_key(&ingestion_id, &history_id, &file_result.file_name);

    match state
        .file_storage
        .upload_file(
            file_key
                .to_str()
                .ok_or(IngestionError::InternalServerError)?,
            file_result.file_content,
        )
        .await
    {
        Ok(()) => {}
        Err(storage_error) => {
            let failure_data = IngestionData::V1(IngestionDataV1::Failed(IngestionFailureData {
                stage: IngestionFailureStage::Storage,
                error_message: format!("Failed to upload file to storage: {storage_error}"),
            }));

            let versioned_update =
                IngestionHistoryVersionUpdate::FailedUpdate { data: failure_data };

            state
                .store
                .create_ingestion_history_version_and_discard_previous(
                    processing_inserted.clone(),
                    versioned_update,
                )
                .await
                .change_context(IngestionError::InternalServerError)?;

            return Err(StorageStageError::UploadFailed {
                reason: storage_error.to_string(),
            }
            .into())
            .switch();
        }
    };

    let success_data = IngestionData::V1(IngestionDataV1::Processed(IngestionProcessedData {
        file_key: file_key.clone(),
        downloaded_at: date_time::now(),
    }));

    // Create Processed version with success data and discard previous version
    let processed_update = IngestionHistoryVersionUpdate::ProcessedUpdate { data: success_data };

    state
        .store
        .create_ingestion_history_version_and_discard_previous(
            processing_inserted,
            processed_update,
        )
        .await
        .change_context(IngestionError::InternalServerError)?;

    utils::notify_transformation_service(state, &ingestion_id, &history_id, file_key).await;

    Ok(())
}

/// Upload file manually to a manual ingestion config
pub async fn upload_file_to_ingestion(
    state: &AppState,
    ingestion_id: String,
    request: ProcessedIngestionFileRequest,
    auth_profile_id: &str,
) -> IngestionResponse<()> {
    let config = state
        .store
        .find_ingestion_config_by_profile_id_and_ingestion_id(auth_profile_id, &ingestion_id)
        .await
        .to_not_found_response(|| IngestionError::IngestionConfigNotFound {
            ingestion_id: ingestion_id.clone(),
        })?;

    if !config.is_active {
        return Err(error_stack::report!(IngestionError::IngestionNotActive));
    }

    let context = config
        .config
        .create_context_for_manual_upload(request.file_data, request.file_name)?;

    process_ingestion(
        state,
        ingestion_id.clone(),
        config.profile_id,
        config.account_id,
        context,
    )
    .await?;

    Ok(ApplicationResponse::StatusOk)
}

/// Download ingested file by history ID
pub async fn download_ingested_file(
    state: &AppState,
    history_id: &str,
    auth_profile_id: &str,
) -> IngestionResponse<()> {
    // Find the ingestion history record
    let history = state
        .store
        .find_ingestion_history_by_id_and_profile(history_id, auth_profile_id)
        .await
        .to_not_found_response(|| IngestionError::IngestionHistoryNotFound {
            history_id: history_id.to_string(),
        })?;

    // Check if the ingestion has processed data (file not available if no data)
    let data = history
        .data
        .ok_or(error_stack::report!(IngestionError::FileNotAvailable))?;

    // Extract file key and filename from processed data
    let file_key = data
        .get_file_key()
        .ok_or(error_stack::report!(IngestionError::FileNotAvailable))?;

    let file_name = data
        .get_file_name()
        .ok_or(error_stack::report!(IngestionError::FileNotAvailable))?;

    // Convert PathBuf to &str for file storage
    let file_key_str = file_key
        .to_str()
        .ok_or(error_stack::report!(IngestionError::InternalServerError))?;

    // Retrieve file from storage
    let file_bytes = state
        .file_storage
        .retrieve_file(file_key_str)
        .await
        .change_context(IngestionError::InternalServerError)?;

    Ok(ApplicationResponse::FileDownload {
        data: file_bytes,
        filename: file_name,
        content_type: mime::APPLICATION_OCTET_STREAM,
    })
}
