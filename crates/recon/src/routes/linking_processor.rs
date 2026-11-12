use actix_web::{web, HttpRequest, Responder};
use api_models::linking_processor::{
    LinkingProcessorStatusRequest, LinkingProcessorStatusResponse, LinkingTriggerResponse,
};
use common_types::permissions::Permission;
use utoipa::OpenApi;

use crate::{
    core,
    services::{
        api,
        auth::{AdminApiAuth, JwtAuth},
    },
};

use super::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(get_processor_status, set_processor_status, enqueue_entry, process_entry),
    components(schemas(LinkingProcessorStatusResponse, LinkingProcessorStatusRequest, LinkingTriggerResponse)),
    tags(
        (name = "Linking Processor", description = "Entry linking processor controls and enqueue endpoints"),
    ),
)]
pub struct LinkingProcessorApi;

#[utoipa::path(
    get,
    path = "/linking_processor/status",
    tag = "Linking Processor",
    responses(
        (status = 200, description = "Current processor status", body = LinkingProcessorStatusResponse),
        (status = 401, description = "Unauthorized - Invalid or missing admin API key"),
        (status = 500, description = "Internal server error"),
    ),
    security(("ApiKey" = [])),
)]
pub async fn get_processor_status(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| async move {
        core::linking_processor::get_processor_status(&state).await
    })
    .await
}

#[utoipa::path(
    post,
    path = "/linking_processor/status",
    tag = "Linking Processor",
    request_body = LinkingProcessorStatusRequest,
    responses(
        (status = 200, description = "Processor status updated"),
        (status = 400, description = "Invalid status value"),
        (status = 401, description = "Unauthorized - Invalid or missing admin API key"),
        (status = 500, description = "Internal server error"),
    ),
    security(("ApiKey" = [])),
)]
pub async fn set_processor_status(
    state: web::Data<AppState>,
    req: web::Json<LinkingProcessorStatusRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| async move {
        core::linking_processor::set_processor_status(state, req.status).await
    })
    .await
}

#[utoipa::path(
    post,
    path = "/linking_processor/{entry_id}/enqueue",
    tag = "Linking Processor",
    params(
        ("entry_id" = String, Path, description = "Entry ID to enqueue for linking")
    ),
    responses(
        (status = 200, description = "Entry linking status and data reset to pending for processing"),
        (status = 400, description = "Invalid entry linking status - only failed entries can be enqueued"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 403, description = "Forbidden - Missing transaction write permission"),
        (status = 404, description = "Entry not found or account does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn enqueue_entry(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::linking_processor::enqueue_entry(state, path.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/linking_processor/{entry_id}/process",
    tag = "Linking Processor",
    params(
        ("entry_id" = String, Path, description = "Physical entry ID to process inline")
    ),
    responses(
        (status = 200, description = "Linking completed or a processing failure was recorded", body = LinkingTriggerResponse),
        (status = 400, description = "Only entries pending linking can be processed"),
        (status = 401, description = "Unauthorized - Invalid or missing admin API key"),
        (status = 404, description = "Entry not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("ApiKey" = [])),
)]
pub async fn process_entry(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| async move {
        Box::pin(core::linking_processor::process_entry(
            state,
            path.into_inner(),
        ))
        .await
    })
    .await
}
