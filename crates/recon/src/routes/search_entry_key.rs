use actix_web::{web, HttpRequest, Responder};
use api_models::search_entry_key::{SearchEntryKeyListRequest, SearchEntryKeyResponse};
use common_types::permissions::Permission;
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{api, auth::JwtAuth},
};

#[utoipa::path(
    post,
    path = "/search_entry_keys",
    tag = "Search Entry Keys",
    request_body = SearchEntryKeyListRequest,
    responses(
        (status = 200, description = "Search entry keys found", body = Vec<SearchEntryKeyResponse>),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn list_search_entry_keys_by_entry_ids(
    state: web::Data<AppState>,
    req: web::Json<SearchEntryKeyListRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::search_entry_key::list_search_entry_keys_by_entry_ids(
                &state,
                auth,
                req.into_inner(),
            )
            .await
        },
    )
    .await
}

#[derive(OpenApi)]
#[openapi(
    paths(list_search_entry_keys_by_entry_ids),
    components(schemas(SearchEntryKeyListRequest, SearchEntryKeyResponse)),
    tags((name = "Search Entry Keys", description = "Search entry key management"))
)]
pub struct SearchEntryKeyApi;
