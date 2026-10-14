use actix_web::{web, HttpRequest, Responder};
use api_models::linking_engine::{LinkingProcessorStatusRequest, LinkingProcessorStatusResponse};
use utoipa::OpenApi;

use crate::{
    core,
    services::{api, auth::AdminApiAuth},
};

use super::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(get_processor_status, set_processor_status),
    components(schemas(LinkingProcessorStatusResponse, LinkingProcessorStatusRequest)),
    tags(
        (name = "Linking Engine", description = "Entry linking engine processing endpoints"),
    ),
)]
pub struct LinkingEngineApi;

#[utoipa::path(
    get,
    path = "/linking_engine/processor/status",
    tag = "Linking Engine",
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
        core::linking_engine::get_processor_status(&state).await
    })
    .await
}

#[utoipa::path(
    post,
    path = "/linking_engine/processor/status",
    tag = "Linking Engine",
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
        core::linking_engine::set_processor_status(state, req.status).await
    })
    .await
}
