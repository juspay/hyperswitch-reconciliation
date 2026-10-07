use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, Responder};
use api_models::ingestion::{
    IngestionConfigCreateRequest, IngestionConfigListResponse, IngestionConfigResponse,
    IngestionConfigUpdateRequest, IngestionFileUploadRequest, IngestionFileUploadResponse,
    IngestionHistoryListResponse, IngestionHistoryResponse,
};
use common_types::{
    ingestion::{ListIngestionConfigsFilters, ListIngestionHistoryFilters},
    permissions::Permission,
};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{
        api,
        auth::{AdminApiAuth, JwtAuth, NoAuth},
    },
    utils,
};

/// OpenAPI documentation for ingestion endpoints
#[derive(OpenApi)]
#[openapi(
    paths(
        create_ingestion_config,
        upload_file_to_ingestion,
        list_ingestion_configs,
        get_ingestion_config,
        update_ingestion_config,
        list_ingestion_history,
        trigger_ingestion,
        download_ingested_file,
    ),
    components(
        schemas(
            IngestionConfigCreateRequest,
            IngestionConfigResponse,
            IngestionConfigListResponse,
            IngestionConfigUpdateRequest,
            IngestionFileUploadRequest,
            IngestionFileUploadResponse,
            IngestionHistoryListResponse,
        )
    ),
    tags(
        (name = "Ingestion", description = "Ingestion configuration and management endpoints"),
    ),
)]
pub struct IngestionApi;

#[utoipa::path(
    post,
    path = "/ingestions/config",
    tag = "Ingestion",
    request_body = IngestionConfigCreateRequest,
    security(
        ("ApiKey" = [])
    ),
    responses(
        (status = 201, description = "Ingestion config created successfully", body = IngestionConfigResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized - invalid or missing admin API key"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_ingestion_config(
    state: web::Data<AppState>,
    req: web::Json<IngestionConfigCreateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| async move {
        core::ingestion::create_ingestion_config(&state, req.into_inner()).await
    })
    .await
}

/// Upload file manually to ingestion
#[utoipa::path(
    post,
    path = "/ingestions/{ingestion_id}/upload",
    tag = "Ingestion",
    params(
        ("ingestion_id" = String, Path, description = "Unique identifier of the manual ingestion config"),
    ),
    request_body(content = IngestionFileUploadRequest, content_type = "multipart/form-data"),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "File uploaded successfully", body = IngestionFileUploadResponse),
        (status = 400, description = "Invalid file or ingestion config not manual"),
        (status = 404, description = "Ingestion config not found"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn upload_file_to_ingestion(
    state: web::Data<AppState>,
    path: web::Path<String>,
    MultipartForm(payload): MultipartForm<IngestionFileUploadRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let ingestion_id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconIngestionWrite),
        &http_req,
        state,
        |state, auth| async move {
            let processed_payload = utils::ingestion::process_multipart_request(payload)?;

            core::ingestion::upload_file_to_ingestion(
                &state,
                ingestion_id,
                processed_payload,
                &auth.profile_id,
            )
            .await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/ingestions/config",
    tag = "Ingestion",
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "List of ingestion configs", body = IngestionConfigListResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_ingestion_configs(
    state: web::Data<AppState>,
    query: web::Query<ListIngestionConfigsFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconIngestionRead),
        &http_req,
        state,
        |state, auth| async move {
            let filters = query.into_inner();
            core::ingestion::list_ingestion_configs(state, &auth.profile_id, filters).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/ingestions/config/{ingestion_id}",
    tag = "Ingestion",
    params(
        ("ingestion_id" = String, Path, description = "Unique identifier of the ingestion config"),
    ),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Ingestion config retrieved successfully", body = IngestionConfigResponse),
        (status = 404, description = "Ingestion config not found"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_ingestion_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let ingestion_id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconIngestionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::ingestion::get_ingestion_config(&state, &auth.profile_id, ingestion_id).await
        },
    )
    .await
}

#[utoipa::path(
    put,
    path = "/ingestions/config/{ingestion_id}",
    tag = "Ingestion",
    params(
        ("ingestion_id" = String, Path, description = "Unique identifier of the ingestion config"),
    ),
    request_body = IngestionConfigUpdateRequest,
    security(
        ("ApiKey" = [])
    ),
    responses(
        (status = 200, description = "Ingestion config updated successfully", body = IngestionConfigResponse),
        (status = 404, description = "Ingestion config not found"),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized - invalid or missing admin API key"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_ingestion_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<IngestionConfigUpdateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let ingestion_id = path.into_inner();
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| async move {
        core::ingestion::update_ingestion_config(&state, ingestion_id, req.into_inner()).await
    })
    .await
}

#[utoipa::path(
    get,
    path = "/ingestions/history/{id}",
    tag = "Ingestion",
    params(
        ("id" = String, Path, description = "Primary key ID of the specific ingestion history version"),
    ),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Ingestion config and history retrieved successfully", body = IngestionHistoryResponse),
        (status = 404, description = "Ingestion history not found - does not exist or does not belong to authenticated user's profile"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn find_ingestion_config_and_history_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconIngestionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::ingestion::find_ingestion_history_with_config_by_id(&state, &id, &auth.profile_id)
                .await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/ingestions/history",
    tag = "Ingestion",
    params(ListIngestionHistoryFilters),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Ingestion history retrieved successfully", body = IngestionHistoryListResponse),
        (status = 404, description = "Ingestion config not found"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_ingestion_history(
    state: web::Data<AppState>,
    query: web::Query<ListIngestionHistoryFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconIngestionRead),
        &http_req,
        state,
        |state, auth| async move {
            let filters = query.into_inner();
            core::ingestion::list_ingestion_history(&state, auth.profile_id, filters).await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/ingestions/{ingestion_id}/trigger",
    tag = "Ingestion",
    params(
        ("ingestion_id" = String, Path, description = "Unique identifier of the ingestion config"),
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Ingestion triggered successfully"),
        (status = 404, description = "Ingestion config not found"),
        (status = 400, description = "Ingestion config is not active, or the payload or credentials failed validation"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn trigger_ingestion(
    state: web::Data<AppState>,
    path: web::Path<String>,
    payload: web::Json<serde_json::Value>,
    http_req: HttpRequest,
) -> impl Responder {
    let ingestion_id = path.into_inner();
    // Extract headers before moving into async closure
    let headers = http_req.headers().clone();

    api::server_wrap(NoAuth, &http_req, state, |state, _auth| async move {
        // Call the unified trigger_ingestion with raw bytes and headers
        core::ingestion::trigger_ingestion(
            &state,
            ingestion_id,
            payload.into_inner(), // Pass raw json directly
            &headers,             // Pass headers for validation
        )
        .await
    })
    .await
}

/// Download ingested file by history ID
#[utoipa::path(
    get,
    path = "/ingestions/history/{history_id}/download",
    tag = "Ingestion",
    params(
        ("history_id" = String, Path, description = "Unique identifier of the ingestion history"),
    ),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "File downloaded successfully", content_type = "application/octet-stream"),
        (status = 404, description = "Ingestion history not found or file not available"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn download_ingested_file(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let history_id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconIngestionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::ingestion::download_ingested_file(&state, &history_id, &auth.profile_id).await
        },
    )
    .await
}
