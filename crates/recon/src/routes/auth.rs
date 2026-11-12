use actix_web::{web, HttpRequest, Responder};
use utoipa::OpenApi;

use crate::{
    core::auth,
    routes::AppState,
    services::{api, auth::AdminApiAuth},
};
use api_models::auth::{
    TestMerchantSetupRequest, TestMerchantSetupResponse, TestMerchantSetupWithIdsRequest,
};

#[utoipa::path(
    post,
    path = "/auth/bootstrap",
    tag = "Auth",
    request_body = TestMerchantSetupRequest,
    security(("ApiKey" = [])),
    responses(
        (status = 200, description = "Bootstrap successful - merchant and profile created", body = TestMerchantSetupResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn bootstrap(
    state: web::Data<AppState>,
    request: web::Json<TestMerchantSetupRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| {
        auth::bootstrap(state, request)
    })
    .await
}

#[utoipa::path(
    post,
    path = "/auth/bootstrap_with_ids",
    tag = "Auth",
    request_body = TestMerchantSetupWithIdsRequest,
    security(("ApiKey" = [])),
    responses(
        (status = 200, description = "Bootstrap successful - merchant and profile created with provided IDs", body = TestMerchantSetupResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Merchant ID or Profile ID already exists"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn bootstrap_with_merchant_id_profile_id(
    state: web::Data<AppState>,
    request: web::Json<TestMerchantSetupWithIdsRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(AdminApiAuth, &http_req, state, |state, _auth| {
        auth::bootstrap_with_merchant_id_profile_id(state, request)
    })
    .await
}

#[derive(OpenApi)]
#[openapi(
    paths(bootstrap, bootstrap_with_merchant_id_profile_id),
    components(schemas(
        TestMerchantSetupRequest,
        TestMerchantSetupResponse,
        TestMerchantSetupWithIdsRequest
    )),
    tags((name = "Auth", description = "Authentication operations")),
)]
pub struct AuthApi;
