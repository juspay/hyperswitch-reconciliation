use actix_web::{web, HttpRequest, Responder};
use api_models::search_expectation::SearchExpectationResponse;
use common_types::permissions::Permission;
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{api, auth::JwtAuth},
};

#[utoipa::path(
    get,
    path = "/search_expectations/primary_transaction/{primary_transaction_id}",
    tag = "Search Expectations",
    params(
        ("primary_transaction_id" = String, Path, description = "Primary transaction ID")
    ),
    responses(
        (status = 200, description = "Search expectations found", body = Vec<SearchExpectationResponse>),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Transaction not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn list_search_expectations_by_primary_transaction_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let primary_transaction_id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::search_expectation::list_search_expectations_by_primary_transaction_id(
                state,
                &primary_transaction_id,
                auth,
            )
            .await
        },
    )
    .await
}

#[derive(OpenApi)]
#[openapi(
    paths(list_search_expectations_by_primary_transaction_id),
    components(schemas(SearchExpectationResponse)),
    tags((name = "Search Expectations", description = "Search expectations management"))
)]
pub struct SearchExpectationApi;
