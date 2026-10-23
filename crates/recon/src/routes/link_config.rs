use actix_web::{web, HttpRequest, Responder};
use api_models::link_config::{
    GetLinkConfigResponse, LinkConfigCreateRequest, LinkConfigUpdateRequest,
};
use common_types::permissions::Permission;
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::JwtAuth},
};

#[derive(OpenApi)]
#[openapi(
    paths(create_link_config, update_link_config, get_link_config_by_id, list_link_configs_by_profile_id),
    components(schemas(LinkConfigCreateRequest, LinkConfigUpdateRequest, GetLinkConfigResponse)),
    tags((name = "Link Configs", description = "Entry linking configuration management endpoints")),
)]
pub struct LinkConfigApi;

#[utoipa::path(
    get,
    path = "/link_configs/{link_config_id}",
    tag = "Link Configs",
    params(("link_config_id" = String, Path, description = "Link config ID")),
    responses(
        (status = 200, description = "Link config retrieved successfully", body = GetLinkConfigResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Link config not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn get_link_config_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let link_config_id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconRuleRead),
        &http_req,
        state,
        |state, auth| async move {
            core::link_config::get_link_config_by_id(&state, &link_config_id, auth).await
        },
    )
    .await
}

/// Lists active and inactive link configs for the authenticated profile.
#[utoipa::path(
    get,
    path = "/link_configs",
    tag = "Link Configs",
    responses(
        (status = 200, description = "Link configs retrieved successfully", body = Vec<GetLinkConfigResponse>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 403, description = "Insufficient permissions"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn list_link_configs_by_profile_id(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconRuleRead),
        &http_req,
        state,
        |state, auth| async move {
            core::link_config::list_link_configs_by_profile_id(&state, &auth.profile_id).await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/link_configs",
    tag = "Link Configs",
    request_body = LinkConfigCreateRequest,
    responses(
        (status = 201, description = "Link config created successfully", body = GetLinkConfigResponse),
        (status = 400, description = "Bad request - validation failed"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Account or recon rule not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn create_link_config(
    state: web::Data<AppState>,
    req: web::Json<LinkConfigCreateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconRuleWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::link_config::create_link_config(&state, req.into_inner(), auth).await
        },
    )
    .await
}

/// Updates supplied fields without rebuilding existing links or search keys.
#[utoipa::path(
    put,
    path = "/link_configs/{link_config_id}",
    tag = "Link Configs",
    params(("link_config_id" = String, Path, description = "Link config ID")),
    request_body = LinkConfigUpdateRequest,
    responses(
        (status = 200, description = "Link config updated successfully", body = GetLinkConfigResponse),
        (status = 400, description = "Bad request - validation failed"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Link config, account, or recon rule not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = [])),
)]
pub async fn update_link_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<LinkConfigUpdateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let link_config_id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconRuleWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::link_config::update_link_config(&state, &link_config_id, req.into_inner(), auth)
                .await
        },
    )
    .await
}
