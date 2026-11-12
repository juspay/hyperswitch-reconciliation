use actix_web::{web, HttpRequest, Responder};
use common_types::permissions::Permission;
use utoipa::OpenApi;

use api_models::recon_engine::{
    ProcessorStatusRequest, ProcessorStatusResponse, ReconTriggerResponse,
};

use crate::{
    core,
    services::{
        api,
        auth::{AdminApiAuth, JwtAuth},
    },
};

use super::AppState;

/// OpenAPI documentation for recon engine endpoints
#[derive(OpenApi)]
#[openapi(
    paths(process_staging_entry, process_duplicate_staging_entry, get_processor_status, set_processor_status),
    components(
        schemas(
            ReconTriggerResponse,
            ProcessorStatusResponse,
            ProcessorStatusRequest
        )
    ),
    tags(
        (name = "Recon Engine", description = "Reconciliation engine processing endpoints"),
    ),
)]
pub struct ReconEngineApi;

#[utoipa::path(
    post,
    path = "/recon_engine/{id}/process",
    tag = "Recon Engine",
    params(
        ("id" = String, Path, description = "Staging entry ID to process")
    ),
    responses(
        (status = 200, description = "Staging entry processed successfully", body = ReconTriggerResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Staging entry not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn process_staging_entry(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionWrite),
        &http_req,
        state,
        |state, auth| async move {
            let id = path.into_inner();
            Box::pin(core::recon_engine::process_staging_entry(state, id, auth)).await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/recon_engine/{id}/process_duplicate",
    tag = "Recon Engine",
    params(
        ("id" = String, Path, description = "Staging entry ID to force process (bypass duplicate checks)")
    ),
    responses(
        (status = 200, description = "Staging entry force processed successfully", body = ReconTriggerResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Staging entry not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn process_duplicate_staging_entry(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionWrite),
        &http_req,
        state,
        |state, auth| async move {
            let id = path.into_inner();
            Box::pin(core::recon_engine::process_duplicate_staging_entry(
                state, id, auth,
            ))
            .await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/recon_engine/processor/status",
    tag = "Recon Engine",
    responses(
        (status = 200, description = "Current processor status", body = ProcessorStatusResponse),
        (status = 500, description = "Internal server error"),
    ),
    security(("ApiKey" = [])),
)]
pub async fn get_processor_status(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| async move {
        core::recon_engine::get_processor_status(&state).await
    })
    .await
}

#[utoipa::path(
    post,
    path = "/recon_engine/processor/status",
    tag = "Recon Engine",
    request_body = ProcessorStatusRequest,
    responses(
        (status = 200, description = "Processor status updated"),
        (status = 400, description = "Invalid status value"),
        (status = 500, description = "Internal server error"),
    ),
    security(("ApiKey" = [])),
)]
pub async fn set_processor_status(
    state: web::Data<AppState>,
    req: web::Json<ProcessorStatusRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| async move {
        core::recon_engine::set_processor_status(state, req.status).await
    })
    .await
}
