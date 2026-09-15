use actix_web::{web, HttpRequest, Responder};
use api_models::business_profile::{
    BusinessProfileCreateRequest, BusinessProfileResponse, ReconStatusResponse,
};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::JwtAuth},
};

/// OpenAPI documentation for business profile endpoints
///
/// This struct defines the OpenAPI documentation for all business profile related endpoints.
/// It's automatically merged into the main API documentation in `crate::openapi::create_openapi()`.
///
/// When adding new endpoints to this module:
/// 1. Add the function name to the `paths()` list
/// 2. Add any new request/response types to the `components(schemas())` list
#[derive(OpenApi)]
#[openapi(
    paths(
        list_business_profiles,
        create_business_profile,
        get_business_profile,
        get_recon_status,
    ),
    components(
        schemas(
            BusinessProfileCreateRequest,
            BusinessProfileResponse,
            ReconStatusResponse,
        )
    ),
    tags(
        (name = "Business Profile", description = "Business profile management endpoints"),
    ),
)]
pub struct BusinessProfileApi;

/// List business profiles for authenticated merchant
#[utoipa::path(
    get,
    path = "/business_profiles",
    tag = "Business Profile",
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Business profiles retrieved successfully", body = Vec<BusinessProfileResponse>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_business_profiles(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move {
            core::business_profile::list_business_profiles_by_merchant_id(state, auth.merchant_id)
                .await
        },
    )
    .await
}

/// Create business profile for authenticated merchant
#[utoipa::path(
    post,
    path = "/business_profiles",
    tag = "Business Profile",
    request_body = BusinessProfileCreateRequest,
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 201, description = "Business profile created successfully", body = BusinessProfileResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_business_profile(
    state: web::Data<AppState>,
    req: web::Json<BusinessProfileCreateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let request = req.into_inner();
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move {
            core::business_profile::create_business_profile(state, auth.merchant_id, request).await
        },
    )
    .await
}

/// Get specific business profile
#[utoipa::path(
    get,
    path = "/business_profiles/{profile_id}",
    tag = "Business Profile",
    params(
        ("profile_id" = String, Path, description = "Unique identifier of the business profile"),
    ),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Business profile retrieved successfully", body = BusinessProfileResponse),
        (status = 400, description = "Business profile does not belong to the authenticated merchant"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 401, description = "Business profile not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_business_profile(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let profile_id = path.into_inner();
    api::server_wrap(
        JwtAuth::with_profile_without_permission(profile_id.clone()),
        &http_req,
        state,
        |state, _auth| async move {
            core::business_profile::get_business_profile(state, profile_id).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/business_profiles/{profile_id}/recon_engine/status",
    tag = "Business Profile",
    params(
        ("profile_id" = String, Path, description = "Unique identifier of the business profile"),
    ),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Recon processor status and count of the profile's pending staging entries", body = ReconStatusResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_recon_status(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let profile_id = path.into_inner();
    api::server_wrap(
        JwtAuth::with_profile_without_permission(profile_id),
        &http_req,
        state,
        |state, auth| async move { core::business_profile::get_recon_status(state, auth).await },
    )
    .await
}
