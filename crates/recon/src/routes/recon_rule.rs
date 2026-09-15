use actix_web::{web, HttpRequest, Responder};
use api_models::recon_rule::{
    GetReconRuleResponse, ReconRuleCreateRequest, ReconRuleUpdateRequest,
};
use common_types::permissions::Permission;
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::JwtAuth},
};

/// OpenAPI documentation for recon rule endpoints
#[derive(OpenApi)]
#[openapi(
    paths(
        create_recon_rule_v2,
        update_recon_rule,
        list_recon_rules_by_profile_id_v2,
        get_recon_rule_v2_by_id,
    ),
    components(
        schemas(
            ReconRuleCreateRequest,
            ReconRuleUpdateRequest,
            GetReconRuleResponse,
        )
    ),
    tags(
        (name = "Recon Rules", description = "Reconciliation rule management endpoints"),
    ),
)]
pub struct ReconRuleApi;

#[utoipa::path(
    post,
    path = "/recon_rules/v2",
    tag = "Recon Rules",
    request_body = ReconRuleCreateRequest,
    responses(
        (status = 201, description = "Recon rule created successfully (v2)", body = GetReconRuleResponse),
        (status = 400, description = "Bad request - validation failed"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn create_recon_rule_v2(
    state: web::Data<AppState>,
    req: web::Json<ReconRuleCreateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconRuleWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::recon_rule::create_recon_rule_v2(&state, req.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/recon_rules/v2/{rule_id}",
    tag = "Recon Rules",
    params(
        ("rule_id" = String, Path, description = "Recon rule ID")
    ),
    responses(
        (status = 200, description = "Recon rule v2 retrieved successfully", body = GetReconRuleResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Recon rule not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn get_recon_rule_v2_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let rule_id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconRuleRead),
        &http_req,
        state,
        |state, auth| async move {
            core::recon_rule::get_recon_rule_v2_by_id(&state, &rule_id, auth).await
        },
    )
    .await
}

#[utoipa::path(
    put,
    path = "/recon_rules/{rule_id}",
    tag = "Recon Rules",
    params(
        ("rule_id" = String, Path, description = "Recon rule ID")
    ),
    request_body = ReconRuleUpdateRequest,
    responses(
        (status = 200, description = "Recon rule updated successfully", body = GetReconRuleResponse),
        (status = 400, description = "Bad request - validation failed"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Recon rule not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn update_recon_rule(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<ReconRuleUpdateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let rule_id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconRuleWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::recon_rule::update_recon_rule(&state, &rule_id, req.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/recon_rules/v2",
    tag = "Recon Rules",
    responses(
        (status = 200, description = "Recon rules retrieved successfully", body = Vec<GetReconRuleResponse>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn list_recon_rules_by_profile_id_v2(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconRuleRead),
        &http_req,
        state,
        |state, auth| async move {
            core::recon_rule::list_recon_rules_by_profile_id_v2(&state, &auth.profile_id).await
        },
    )
    .await
}
