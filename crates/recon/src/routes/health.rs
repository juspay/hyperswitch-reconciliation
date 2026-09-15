use actix_web::{web, Responder};
use utoipa::OpenApi;

use super::AppState;

/// OpenAPI documentation for health check endpoints
///
/// This struct defines the OpenAPI documentation for health check endpoints.
/// It's automatically merged into the main API documentation in `crate::openapi::create_openapi()`.
#[derive(OpenApi)]
#[openapi(
    paths(
        get_health,
    ),
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
pub async fn get_health(_state: web::Data<AppState>) -> impl Responder {
    actix_web::HttpResponse::Ok().body("OK")
}
