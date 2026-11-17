use actix_web::{web, HttpRequest, Responder};
use api_models::health::{ComponentHealth, DeepHealthResponse};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::NoAuth},
};

/// OpenAPI documentation for health check endpoints
///
/// This struct defines the OpenAPI documentation for health check endpoints.
/// It's automatically merged into the main API documentation in `crate::openapi::create_openapi()`.
#[derive(OpenApi)]
#[openapi(
    paths(
        get_health,
        get_deep_health,
    ),
    components(schemas(DeepHealthResponse, ComponentHealth)),
    tags(
        (name = "Health", description = "Health check endpoints"),
    ),
)]
pub struct HealthApi;

#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = String),
    ),
)]
pub async fn get_health(state: web::Data<AppState>, http_req: HttpRequest) -> impl Responder {
    api::server_wrap(NoAuth, &http_req, state, |_state, _auth| async move {
        core::health::health_check().await
    })
    .await
}

#[utoipa::path(
    get,
    path = "/health/deep",
    tag = "Health",
    responses(
        (status = 200, description = "All dependencies are healthy", body = DeepHealthResponse),
        (status = 503, description = "At least one dependency is unhealthy", body = DeepHealthResponse),
    ),
)]
pub async fn get_deep_health(state: web::Data<AppState>, http_req: HttpRequest) -> impl Responder {
    api::server_wrap(NoAuth, &http_req, state, |state, _auth| async move {
        core::health::deep_health_check(state).await
    })
    .await
}
