use actix_web::{web, HttpRequest, Responder};
use api_models::entry::{ApiEntryListFilters, EntryListPage, GetEntryResponse, ListEntriesRequest};
use common_types::{
    date_time::TimeRange,
    entry::{EntryEffectiveAtSortField, EntryListSortField, ListEntriesFilters},
    pagination::{CursorDirection, SortOrder},
    permissions::Permission,
};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::JwtAuth},
};

/// OpenAPI documentation for entry endpoints
#[derive(OpenApi)]
#[openapi(
    paths(
        get_entry_by_id,
        list_entries,
        list_entries_by_account_id,
        list_entries_by_transaction_id,
    ),
    components(
        schemas(
            GetEntryResponse,
            common_types::amount::AmountResponse,
            common_enums::BalanceDirection,
            common_enums::EntryStatus,
            common_enums::Currency,
            ListEntriesFilters,
            ApiEntryListFilters,
            ListEntriesRequest,
            EntryListSortField,
            EntryEffectiveAtSortField,
            SortOrder,
            CursorDirection,
            EntryListPage,
            TimeRange,
        )
    ),
    tags(
        (name = "Entries", description = "Entry management endpoints"),
    ),
)]
pub struct EntryApi;

#[utoipa::path(
    get,
    path = "/entries/{entry_id}",
    tag = "Entries",
    params(
        ("entry_id" = String, Path, description = "Entry ID")
    ),
    responses(
        (status = 200, description = "Entry found", body = GetEntryResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Entry not found or account does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn get_entry_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            let entry_id = path.into_inner();
            core::entry::get_entry_by_id(state, entry_id, auth).await
        },
    )
    .await
}

/// Get entries by account ID
#[utoipa::path(
    get,
    path = "/accounts/{account_id}/entries",
    tag = "Account",
    params(ListEntriesFilters),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Entries retrieved successfully", body = Vec<GetEntryResponse>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Account not found - Account does not exist or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_entries_by_account_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
    filters: web::Query<ListEntriesFilters>,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            let account_id = path.into_inner();
            core::entry::list_entries_by_account_id(state, account_id, auth, filters.into_inner())
                .await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/transactions/{transaction_id}/entries",
    tag = "Entries",
    params(
        ("transaction_id" = String, Path, description = "Transaction ID")
    ),
    responses(
        (status = 200, description = "Entries found - only entries on accounts belonging to the authenticated user's profile are returned; an unknown or foreign transaction id yields an empty list", body = Vec<GetEntryResponse>),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn list_entries_by_transaction_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            let transaction_id = path.into_inner();
            core::entry::list_entries_by_transaction_id(state, transaction_id, auth).await
        },
    )
    .await
}

/// List one transaction version's entries with cursor pagination
#[utoipa::path(
    post,
    path = "/entries/list",
    tag = "Entries",
    request_body = ListEntriesRequest,
    responses(
        (status = 200, description = "Paginated entries", body = EntryListPage),
        (status = 400, description = "Bad request - invalid cursor or filter validation error"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "No transaction with `primary_transaction_id` in the caller's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn list_entries(
    state: web::Data<AppState>,
    req: web::Json<ListEntriesRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move { core::entry::list_entries(state, req.into_inner(), auth).await },
    )
    .await
}
