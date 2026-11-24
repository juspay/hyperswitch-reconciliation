use actix_web::{web, Responder};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::api,
    types::health::{ComponentHealth, DeepHealthResponse, HealthComponent},
};

#[derive(OpenApi)]
#[openapi(
    paths(get_health, get_deep_health),
    components(schemas(DeepHealthResponse, ComponentHealth, HealthComponent)),
    tags((name = "Health", description = "Health check endpoints")),
)]
pub struct HealthApi;

#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is up", body = String),
    ),
)]
pub async fn get_health(state: web::Data<AppState>) -> impl Responder {
    api::server_wrap(state, |_state| core::health::health_check()).await
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
pub async fn get_deep_health(state: web::Data<AppState>) -> impl Responder {
    api::server_wrap(state, core::health::deep_health_check).await
}
