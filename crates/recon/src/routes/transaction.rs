use actix_web::{web, HttpRequest, Responder};
use api_models::{
    bulk_operations::{
        BulkOperationResultItem, TransactionBulkAction, TransactionBulkOperationRequest,
        TransactionBulkSelection,
    },
    transaction::{
        ApiTransactionAccountFilters, GetTransactionResponse, ListTransactionsV2Request,
        TransactionEntry, TransactionListResponse, TransactionListV2Page, TransactionRequest,
        TransactionWithEntriesResponse, VoidTransactionRequest,
    },
};
use common_enums::BalanceDirection;
use common_types::{
    date_time::TimeRange,
    pagination::{CursorDirection, SortOrder},
    permissions::Permission,
    transaction::{TransactionEffectiveAtSortField, TransactionListSortField},
};
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{api, auth::JwtAuth},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        create_transaction,
        get_transaction,
        list_transactions,
        list_transactions_v2,
        execute_transaction_bulk_operations,
        void_transaction,
    ),
    components(
        schemas(
            TransactionEntry,
            TransactionRequest,
            VoidTransactionRequest,
            GetTransactionResponse,
            ApiTransactionAccountFilters,
            TransactionListResponse,
            TransactionWithEntriesResponse,
            BalanceDirection,
            TimeRange,
            TransactionBulkOperationRequest,
            TransactionBulkAction,
            TransactionBulkSelection,
            BulkOperationResultItem,
            ListTransactionsV2Request,
            TransactionListSortField,
            TransactionEffectiveAtSortField,
            SortOrder,
            CursorDirection,
            TransactionListV2Page,
        )
    ),
    tags(
        (name = "Transactions", description = "Transaction management endpoints"),
    ),
)]
pub struct TransactionApi;

/// Create a new transaction
#[utoipa::path(
    post,
    path = "/transactions",
    tag = "Transactions",
    request_body = TransactionRequest,
    security(("Bearer" = [])),
    responses(
        (status = 201, description = "Transaction created successfully", body = GetTransactionResponse),
        (status = 400, description = "Bad request - Invalid amount, entry data, or validation error"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Account or entry not found - One or more accounts/entries do not exist or do not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_transaction(
    state: web::Data<AppState>,
    req: web::Json<TransactionRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::transaction::create_transaction(&state, req.into_inner(), auth).await
        },
    )
    .await
}

/// List transactions with detailed account/entry information
#[utoipa::path(
    get,
    path = "/transactions",
    tag = "Transactions",
    security(("Bearer" = [])),
    params(ApiTransactionAccountFilters),
    responses(
        (status = 200, description = "Transactions with account details retrieved successfully", body = TransactionListResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_transactions(
    state: web::Data<AppState>,
    query: web::Query<ApiTransactionAccountFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::transaction::list_transactions(&state, auth, query.into_inner()).await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/transactions/v2/list",
    tag = "Transactions",
    request_body = ListTransactionsV2Request,
    security(("Bearer" = [])),
    responses(
        (status = 200, description = "Paginated transactions", body = TransactionListV2Page),
        (status = 400, description = "Bad request - invalid cursor or filter validation error"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_transactions_v2(
    state: web::Data<AppState>,
    req: web::Json<ListTransactionsV2Request>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::transaction::list_transactions_v2(&state, auth, req.into_inner()).await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/transactions/bulk_operations",
    tag = "Transactions",
    security(("Bearer" = [])),
    request_body = TransactionBulkOperationRequest,
    responses(
        (status = 200, description = "Bulk operation processed", body = Vec<BulkOperationResultItem>),
        (status = 400, description = "Bad request - Invalid selection or action payload"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn execute_transaction_bulk_operations(
    state: web::Data<AppState>,
    req: web::Json<TransactionBulkOperationRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::bulk_operations::execute_transaction_bulk_operation(state, req.into_inner(), auth)
                .await
        },
    )
    .await
}

/// Get a specific transaction by ID
#[utoipa::path(
    get,
    path = "/transactions/{id}",
    tag = "Transactions",
    security(("Bearer" = [])),
    params(
        ("id" = String, Path, description = "Transaction primary key")
    ),
    responses(
        (status = 200, description = "Transaction retrieved successfully", body = GetTransactionResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Transaction not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_transaction(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move { core::transaction::get_transaction(&state, auth, id).await },
    )
    .await
}

/// Void a specific transaction by ID
#[utoipa::path(
    put,
    path = "/transactions/{id}/void",
    tag = "Transactions",
    request_body = VoidTransactionRequest,
    security(("Bearer" = [])),
    params(
        ("id" = String, Path, description = "Transaction primary ID")
    ),
    responses(
        (status = 200, description = "Transaction voided successfully", body = GetTransactionResponse),
        (status = 400, description = "Bad request - Invalid request or validation error"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Transaction not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn void_transaction(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<VoidTransactionRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::transaction::void_transaction(&state, id, req.into_inner(), auth).await
        },
    )
    .await
}
