use actix_web::{web, HttpRequest, Responder};
use api_models::api_key::{
    ApiKeyResponse, CreateApiKeyRequest, CreateApiKeyResponse, ListApiKeysResponse,
    UpdateApiKeyRequest,
};
use common_types::permissions::Permission;
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{api, auth::JwtOrAdminApiAuth},
};

#[utoipa::path(
    post,
    path = "/api_keys",
    tag = "API Keys",
    params(
        ("x-merchant-id" = Option<String>, Header, description = "Required with the admin API key; names the merchant")
    ),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created. The key is only shown in this response", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []), ("ApiKey" = []))
)]
pub async fn create_api_key(
    state: web::Data<AppState>,
    req: web::Json<CreateApiKeyRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtOrAdminApiAuth::new(Permission::MerchantReconApiKeyWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::api_key::create_api_key(state, req.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api_keys",
    tag = "API Keys",
    responses(
        (status = 200, description = "Active API keys of the merchant, newest first", body = ListApiKeysResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []), ("ApiKey" = []))
)]
pub async fn list_api_keys(state: web::Data<AppState>, http_req: HttpRequest) -> impl Responder {
    api::server_wrap(
        JwtOrAdminApiAuth::new(Permission::MerchantReconApiKeyRead),
        &http_req,
        state,
        |state, auth| async move { core::api_key::list_api_keys(state, auth).await },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api_keys/{api_key_id}",
    tag = "API Keys",
    params(
        ("api_key_id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key found", body = ApiKeyResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "API key not found, revoked, or belongs to another merchant"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []), ("ApiKey" = []))
)]
pub async fn get_api_key(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let api_key_id = path.into_inner();
    api::server_wrap(
        JwtOrAdminApiAuth::new(Permission::MerchantReconApiKeyRead),
        &http_req,
        state,
        |state, auth| async move { core::api_key::get_api_key(state, &api_key_id, auth).await },
    )
    .await
}

#[utoipa::path(
    put,
    path = "/api_keys/{api_key_id}",
    tag = "API Keys",
    params(
        ("api_key_id" = String, Path, description = "API key ID")
    ),
    request_body = UpdateApiKeyRequest,
    responses(
        (status = 200, description = "API key updated", body = ApiKeyResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "API key not found, revoked, or belongs to another merchant"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []), ("ApiKey" = []))
)]
pub async fn update_api_key(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<UpdateApiKeyRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let api_key_id = path.into_inner();
    api::server_wrap(
        JwtOrAdminApiAuth::new(Permission::MerchantReconApiKeyWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::api_key::update_api_key(state, &api_key_id, req.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    delete,
    path = "/api_keys/{api_key_id}",
    tag = "API Keys",
    params(
        ("api_key_id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key revoked. It stops working on the next request", body = ApiKeyResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "API key not found, already revoked, or belongs to another merchant"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []), ("ApiKey" = []))
)]
pub async fn revoke_api_key(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let api_key_id = path.into_inner();
    api::server_wrap(
        JwtOrAdminApiAuth::new(Permission::MerchantReconApiKeyWrite),
        &http_req,
        state,
        |state, auth| async move { core::api_key::revoke_api_key(state, &api_key_id, auth).await },
    )
    .await
}

#[derive(OpenApi)]
#[openapi(
    paths(
        create_api_key,
        list_api_keys,
        get_api_key,
        update_api_key,
        revoke_api_key
    ),
    components(schemas(
        CreateApiKeyRequest,
        CreateApiKeyResponse,
        UpdateApiKeyRequest,
        ApiKeyResponse,
        ListApiKeysResponse
    )),
    tags((name = "API Keys", description = "Merchant API key management"))
)]
pub struct ApiKeyApi;
