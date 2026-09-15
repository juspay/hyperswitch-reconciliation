use actix_web::{web, HttpRequest, Responder};
use api_models::audit_trail::{AuditEvent, AuditTrailFilters};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::JwtAuth},
};

/// OpenAPI documentation for audit trail endpoints
#[derive(OpenApi)]
#[openapi(
    paths(get_audit_trail),
    components(schemas(AuditTrailFilters, AuditEvent)),
    tags((name = "Audit Trail", description = "Audit trail endpoints")),
)]
pub struct AuditTrailApi;

#[utoipa::path(
    get,
    path = "/audit_trail",
    tag = "Audit Trail",
    params(AuditTrailFilters),
    responses(
        (status = 200, description = "Audit trail events retrieved successfully", body = Vec<AuditEvent>),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt_auth" = []))
)]
pub async fn get_audit_trail(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    query: web::Query<AuditTrailFilters>,
) -> impl Responder {
    let filters = query.into_inner();
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move {
            core::audit_trail::get_audit_trail(&state, auth, filters).await
        },
    )
    .await
}
