use actix_web::{web, HttpRequest, Responder};
use api_models::overview::{
    AccountStagingEntriesOverview, AccountStatusOverview, AccountTransactionStatusAmount,
    OverviewFilters, RuleAccountsOverview, RuleAccountsOverviewResponse, RuleTransactionsOverview,
    RuleTransactionsTimeSeries, StagingEntriesOverviewResponse, StagingEntryDetailedStatusAmount,
    StagingEntryOverviewFilters, StagingEntryStatusAmount, TimeSeriesFilters,
    TransactionStatusAmount, TransactionTimeBucket, TransactionsOverviewResponse,
    TransactionsTimeSeriesResponse,
};
use api_models::staging_entry::{ApiStagingEntryCoarseStatus, ApiStagingEntryStatus};
use api_models::transaction::ApiTransactionFilterStatus;
use common_enums::{BalanceDirection, Currency, Granularity, ReconRuleAccountType};
use common_types::{date_time::TimeRange, permissions::Permission};
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{api, auth::JwtAuth},
};

/// OpenAPI documentation for overview endpoints
#[derive(OpenApi)]
#[openapi(
    paths(
        get_transactions_overview,
        get_transactions_time_series,
        get_transactions_rule_account_breakdown,
        get_staging_entries_overview,
    ),
    components(
        schemas(
            OverviewFilters,
            TimeSeriesFilters,
            Granularity,
            RuleTransactionsOverview,
            TransactionStatusAmount,
            TransactionTimeBucket,
            RuleTransactionsTimeSeries,
            AccountStatusOverview,
            AccountTransactionStatusAmount,
            RuleAccountsOverview,
            TimeRange,
            ApiTransactionFilterStatus,
            Currency,
            StagingEntryOverviewFilters,
            AccountStagingEntriesOverview,
            StagingEntryStatusAmount,
            StagingEntryDetailedStatusAmount,
            ApiStagingEntryStatus,
            ApiStagingEntryCoarseStatus,
            BalanceDirection,
            ReconRuleAccountType,
        )
    ),
    tags(
        (name = "Overview", description = "Profile-level aggregated views over transactions and entries"),
    ),
)]
pub struct OverviewApi;

/// Get whole-range transaction status counts and amount sums grouped by recon rule
#[utoipa::path(
    get,
    path = "/overview/transactions",
    tag = "Overview",
    security(("Bearer" = [])),
    params(OverviewFilters),
    responses(
        (status = 200, description = "Transaction status counts grouped by rule retrieved successfully", body = TransactionsOverviewResponse),
        (status = 400, description = "Bad request - Invalid time range"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_transactions_overview(
    state: web::Data<AppState>,
    query: web::Query<OverviewFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::overview::get_transactions_overview(&state, auth, query.into_inner()).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/overview/transactions/time_series",
    tag = "Overview",
    security(("Bearer" = [])),
    params(TimeSeriesFilters),
    responses(
        (status = 200, description = "Time-bucketed transaction status counts grouped by rule retrieved successfully", body = TransactionsTimeSeriesResponse),
        (status = 400, description = "Bad request - Invalid time range or missing granularity"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_transactions_time_series(
    state: web::Data<AppState>,
    query: web::Query<TimeSeriesFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::overview::get_transactions_time_series(&state, auth, query.into_inner()).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/overview/transactions/rule_account_breakdown",
    tag = "Overview",
    security(("Bearer" = [])),
    params(OverviewFilters),
    responses(
        (status = 200, description = "Transaction status counts grouped by rule and configured account retrieved successfully", body = RuleAccountsOverviewResponse),
        (status = 400, description = "Bad request - Invalid time range"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_transactions_rule_account_breakdown(
    state: web::Data<AppState>,
    query: web::Query<OverviewFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::overview::get_transactions_rule_account_breakdown(
                &state,
                auth,
                query.into_inner(),
            )
            .await
        },
    )
    .await
}

/// Get whole-range staging entry status counts and amount sums grouped by account
#[utoipa::path(
    get,
    path = "/overview/staging_entries",
    tag = "Overview",
    security(("Bearer" = [])),
    params(StagingEntryOverviewFilters),
    responses(
        (status = 200, description = "Staging entry status counts grouped by account retrieved successfully", body = StagingEntriesOverviewResponse),
        (status = 400, description = "Bad request - Invalid time range"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_staging_entries_overview(
    state: web::Data<AppState>,
    query: web::Query<StagingEntryOverviewFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryRead),
        &http_req,
        state,
        |state, auth| async move {
            core::overview::get_staging_entries_overview(&state, auth, query.into_inner()).await
        },
    )
    .await
}
