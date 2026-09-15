use actix_web::{web, HttpRequest, Responder};
use api_models::reporting::{ExceptionReportFilters, TransactionReportFilters};
use common_types::permissions::Permission;
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{api, auth::JwtAuth},
};

#[derive(OpenApi)]
#[openapi(
    tags((name = "Reports", description = "Report generation")),
    paths(trigger_transaction_report, trigger_exception_report),
    components(schemas(TransactionReportFilters, ExceptionReportFilters)),
)]
pub struct ReportApi;

/// Trigger a transaction report
#[utoipa::path(
    post,
    path = "/report/transactions",
    tag = "Reports",
    request_body = TransactionReportFilters,
    responses(
        (status = 200, description = "Transaction report triggered"),
        (status = 400, description = "Invalid time range"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Recon rule not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn trigger_transaction_report(
    state: web::Data<AppState>,
    req: web::Json<TransactionReportFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransactionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::reporting::generate_transaction_report(state, auth, req.into_inner()).await
        },
    )
    .await
}

/// Trigger an exception report
#[utoipa::path(
    post,
    path = "/report/exceptions",
    tag = "Reports",
    request_body = ExceptionReportFilters,
    responses(
        (status = 200, description = "Exception report triggered"),
        (status = 400, description = "Invalid time range"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Recon rule not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn trigger_exception_report(
    state: web::Data<AppState>,
    req: web::Json<ExceptionReportFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::reporting::generate_exception_report(state, auth, req.into_inner()).await
        },
    )
    .await
}
