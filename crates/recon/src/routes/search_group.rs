use actix_web::{web, HttpRequest, Responder};
use api_models::search_group::SearchGroupResponse;
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{api, auth::JwtAuth},
};

#[utoipa::path(
    get,
    path = "/search_groups/{search_group_id}",
    tag = "Search Groups",
    params(
        ("search_group_id" = String, Path, description = "Search group ID")
    ),
    responses(
        (status = 200, description = "Search group found", body = SearchGroupResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Search group not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn get_search_group_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move { core::search_group::get_search_group_by_id(state, &id, auth).await },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/search_groups/primary_transaction/{primary_transaction_id}",
    tag = "Search Groups",
    params(
        ("primary_transaction_id" = String, Path, description = "Primary transaction ID")
    ),
    responses(
        (status = 200, description = "Search group found", body = SearchGroupResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Search group not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn get_search_group_by_primary_transaction_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let primary_transaction_id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic_without_permission(),
        &http_req,
        state,
        |state, auth| async move {
            core::search_group::get_search_group_by_primary_transaction_id(
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
    paths(get_search_group_by_id, get_search_group_by_primary_transaction_id),
    components(schemas(SearchGroupResponse)),
    tags((name = "Search Groups", description = "Search groups management"))
)]
pub struct SearchGroupApi;
