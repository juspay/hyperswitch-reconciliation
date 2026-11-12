use actix_web::{web, HttpRequest, Responder};
use api_models::merchant_account::{MerchantAccountCreateRequest, MerchantAccountResponse};
use common_utils::id_type::MerchantId;
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{
        api,
        auth::{AdminApiAuth, JwtAuth},
    },
};

/// OpenAPI documentation for merchant account endpoints
///
/// This struct defines the OpenAPI documentation for all merchant account related endpoints.
/// It's automatically merged into the main API documentation in `crate::openapi::create_openapi()`.
///
/// When adding new endpoints to this module:
/// 1. Add the function name to the `paths()` list
/// 2. Add any new request/response types to the `components(schemas())` list
#[derive(OpenApi)]
#[openapi(
    paths(
        create_merchant_account,
        get_merchant_account,
    ),
    components(
        schemas(
            MerchantAccountCreateRequest,
            MerchantAccountResponse,
        )
    ),
    tags(
        (name = "Merchant Account", description = "Merchant account management endpoints"),
    ),
)]
pub struct MerchantAccountApi;

#[utoipa::path(
    post,
    path = "/merchant_accounts",
    tag = "Merchant Account",
    request_body = MerchantAccountCreateRequest,
    security(("ApiKey" = [])),
    responses(
        (status = 201, description = "Merchant account created successfully", body = MerchantAccountResponse),
        (status = 409, description = "Merchant account already exists"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_merchant_account(
    state: web::Data<AppState>,
    req: web::Json<MerchantAccountCreateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| async move {
        core::merchant_account::create_merchant_account(state, req.into_inner()).await
    })
    .await
}

#[utoipa::path(
    get,
    path = "/merchant_accounts/{merchant_id}",
    tag = "Merchant Account",
    params(
        ("merchant_id" = String, Path, description = "Unique identifier of the merchant account"),
    ),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Merchant account retrieved successfully", body = MerchantAccountResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token, or merchant_id mismatch"),
        (status = 401, description = "Merchant account not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_merchant_account(
    state: web::Data<AppState>,
    merchant_id: web::Path<String>,
    http_req: HttpRequest,
) -> Result<impl Responder, actix_web::Error> {
    let merchant_id = MerchantId::wrap(merchant_id.into_inner())
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid merchant_id format"))?;
    Ok(api::server_wrap(
        JwtAuth::with_merchant_without_permission(merchant_id.clone()),
        &http_req,
        state,
        move |state, auth| async move {
            core::merchant_account::get_merchant_account(state, &auth.merchant_id).await
        },
    )
    .await)
}
