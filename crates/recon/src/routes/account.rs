use actix_web::{web, HttpRequest, Responder};
use api_models::{
    account::{AccountCreateRequest, AccountUpdateRequest, GetAccountResponse},
    entry::GetEntryResponse,
    staging_entry::GetStagingEntryResponse,
};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::JwtAuth},
};

/// OpenAPI documentation for account endpoints
///
/// This struct defines the OpenAPI documentation for all account related endpoints.
/// It's automatically merged into the main API documentation in `crate::openapi::create_openapi()`.
///
/// When adding new endpoints to this module:
/// 1. Add the function name to the `paths()` list
/// 2. Add any new request/response types to the `components(schemas())` list
#[derive(OpenApi)]
#[openapi(
    paths(
        create_account,
        get_account,
        update_account,
        list_accounts_by_profile_id,
    ),
    components(
        schemas(
            AccountCreateRequest,
            AccountUpdateRequest,
            GetAccountResponse,
            GetEntryResponse,
            GetStagingEntryResponse,
        )
    ),
    tags(
        (name = "Account", description = "Account management endpoints"),
    ),
)]
pub struct AccountApi;

#[utoipa::path(
    post,
    path = "/accounts",
    tag = "Account",
    request_body = AccountCreateRequest,
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 201, description = "Account created successfully", body = GetAccountResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_account(
    state: web::Data<AppState>,
    req: web::Json<AccountCreateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let req = req.into_inner();
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move {
            core::account::create_account(&state, req, &auth.profile_id).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/accounts/{account_id}",
    tag = "Account",
    params(
        ("account_id" = String, Path, description = "Unique identifier of the account"),
    ),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Account retrieved successfully", body = GetAccountResponse),
        (status = 404, description = "Account not found"),
        (status = 400, description = "Invalid account ID provided"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_account(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let account_id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move {
            core::account::get_account(&state, account_id, &auth.profile_id).await
        },
    )
    .await
}

#[utoipa::path(
    put,
    path = "/accounts/{account_id}",
    tag = "Account",
    params(
        ("account_id" = String, Path, description = "Unique identifier of the account"),
    ),
    request_body = AccountUpdateRequest,
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Account updated successfully", body = GetAccountResponse),
        (status = 404, description = "Account not found"),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_account(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<AccountUpdateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let account_id = path.into_inner();
    let req = req.into_inner();
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move {
            core::account::update_account(&state, account_id, req, &auth.profile_id).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/accounts",
    tag = "Account",
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Accounts retrieved successfully", body = Vec<GetAccountResponse>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_accounts_by_profile_id(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move {
            core::account::list_accounts_by_profile_id(&state, &auth.profile_id).await
        },
    )
    .await
}
