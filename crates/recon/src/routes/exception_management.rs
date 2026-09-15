use actix_web::{web, HttpRequest, Responder};
use api_models::{
    staging_entry::{
        ApiLinkableStagingEntryListFiltersV2, GetStagingEntryResponse,
        LinkableStagingEntryListV2Page, ListLinkableStagingEntriesV2Request,
    },
    transaction::{ForceReconcileRequest, GetTransactionResponse, ManualReconciliationRequest},
    transformation::{ValidateEntryDataRequest, ValidateEntryDataResponse},
    user::User,
};
use common_types::{
    pagination::{CursorDirection, SortOrder},
    permissions::Permission,
    staging_entry::{StagingEntryEffectiveAtSortField, StagingEntryListSortField},
};
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{api, auth::JwtAuth},
};

#[allow(deprecated)]
#[derive(OpenApi)]
#[openapi(
    tags((name = "Exception Management", description = "Exception management")),
    paths(
        get_staging_entry_resolutions,
        get_transaction_resolutions,
        force_reconcile_transaction,
        list_linkable_staging_entries_for_transaction,
        list_linkable_staging_entries_for_transaction_v2,
        manual_transaction_reconciliation,
        validate_raw_entry_data,
        list_assignees,
    ),
    components(
        schemas(
            common_enums::ResolutionActionType,
            ForceReconcileRequest,
            GetTransactionResponse,
            GetStagingEntryResponse,
            ApiLinkableStagingEntryListFiltersV2,
            ListLinkableStagingEntriesV2Request,
            StagingEntryListSortField,
            StagingEntryEffectiveAtSortField,
            SortOrder,
            CursorDirection,
            LinkableStagingEntryListV2Page,
            ManualReconciliationRequest,
            ValidateEntryDataRequest,
            User,
        )
    ),
)]
pub struct ExceptionManagementApi;

/// Get available resolution action types for a staging entry
#[utoipa::path(
    get,
    path = "/exception_management/staging_entries/{id}/resolutions",
    tag = "Exception Management",
    params(
        ("id" = String, Path, description = "Staging entry primary ID")
    ),
    responses(
        (status = 200, description = "Available resolution action types", body = Vec<common_enums::ResolutionActionType>),
        (status = 404, description = "Staging entry not found or account does not belong to authenticated user's profile"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn get_staging_entry_resolutions(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionRead),
        &http_req,
        state,
        |state, auth| async move {
            let id = path.into_inner();
            core::exception_management::get_staging_entry_resolutions(state, id, auth).await
        },
    )
    .await
}

/// Get available resolution action types for a transaction
#[utoipa::path(
    get,
    path = "/exception_management/transactions/{id}/resolutions",
    tag = "Exception Management",
    params(
        ("id" = String, Path, description = "Transaction primary ID")
    ),
    responses(
        (status = 200, description = "Available resolution action types", body = Vec<common_enums::ResolutionActionType>),
        (status = 404, description = "Transaction not found"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn get_transaction_resolutions(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionRead),
        &http_req,
        state,
        |state, auth| async move {
            let id = path.into_inner();
            core::exception_management::get_transaction_resolutions(&state, id, auth).await
        },
    )
    .await
}

/// Force reconcile a mismatched transaction by ignoring specified non-critical field mismatches
#[utoipa::path(
    put,
    path = "/exception_management/transactions/{id}/force_reconcile",
    tag = "Exception Management",
    request_body = ForceReconcileRequest,
    security(("Bearer" = [])),
    params(
        ("id" = String, Path, description = "Transaction primary ID")
    ),
    responses(
        (status = 200, description = "Transaction force reconciled successfully", body = GetTransactionResponse),
        (status = 400, description = "Bad request - Invalid request, validation error, or transaction not in mismatched state"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Transaction not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn force_reconcile_transaction(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<ForceReconcileRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::exception_management::force_reconcile_transaction(
                &state,
                id,
                req.into_inner(),
                auth,
            )
            .await
        },
    )
    .await
}

/// Get staging entries that can be linked to a transaction
#[deprecated(note = "use the cursor-paginated v2 endpoint")]
#[utoipa::path(
    get,
    path = "/exception_management/transactions/{id}/linkable_staging_entries",
    tag = "Exception Management",
    params(
        ("id" = String, Path, description = "Transaction primary ID"),
    ),
    responses(
        (status = 200, description = "Linkable staging entries retrieved successfully", body = Vec<GetStagingEntryResponse>),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Transaction not found or doesn't belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn list_linkable_staging_entries_for_transaction(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionRead),
        &http_req,
        state,
        |state, auth| async move {
            let id = path.into_inner();
            core::exception_management::list_linkable_staging_entries_for_transaction(
                &state, id, auth,
            )
            .await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/exception_management/transactions/{id}/linkable_staging_entries/v2/list",
    tag = "Exception Management",
    request_body = ListLinkableStagingEntriesV2Request,
    params(
        ("id" = String, Path, description = "Transaction primary ID"),
    ),
    responses(
        (status = 200, description = "Paginated linkable staging entries", body = LinkableStagingEntryListV2Page),
        (status = 400, description = "Bad request - invalid cursor or filter validation error"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Transaction not found or doesn't belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn list_linkable_staging_entries_for_transaction_v2(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<ListLinkableStagingEntriesV2Request>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::exception_management::list_linkable_staging_entries_for_transaction_v2(
                &state,
                path.into_inner(),
                req.into_inner(),
                auth,
            )
            .await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/exception_management/transactions/{id}/manual_reconciliation",
    tag = "Exception Management",
    request_body = ManualReconciliationRequest,
    security(("Bearer" = [])),
    params(
        ("id" = String, Path, description = "Transaction primary ID to manually reconcile")
    ),
    responses(
        (status = 200, description = "Transaction reconciliation complete", body = GetTransactionResponse),
        (status = 400, description = "Bad request - Invalid request or validation error"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Transaction not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn manual_transaction_reconciliation(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<ManualReconciliationRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::exception_management::manual_transaction_reconciliation(
                &state,
                id,
                req.into_inner(),
                auth,
            )
            .await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/exception_management/validate_entry_data",
    tag = "Exception Management",
    request_body = ValidateEntryDataRequest,
    security(("Bearer" = [])),
    responses(
        (status = 200, description = "Entry data validated Successfully", body = ValidateEntryDataResponse),
        (status = 400, description = "Bad request - Invalid request or validation error"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 404, description = "Metadata schema not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn validate_raw_entry_data(
    state: web::Data<AppState>,
    req: web::Json<ValidateEntryDataRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionRead),
        &http_req,
        state,
        |state, auth| async move {
            core::exception_management::validate_raw_entry_data(&state, req.into_inner(), auth)
                .await
        },
    )
    .await
}

/// List users who can be assigned to exception management tasks
#[utoipa::path(
    get,
    path = "/exception_management/assignees",
    tag = "Exception Management",
    responses(
        (status = 200, description = "List of assignable users", body = Vec<User>),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn list_assignees(state: web::Data<AppState>, http_req: HttpRequest) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconExceptionRead),
        &http_req,
        state,
        |state, auth| async move { core::exception_management::list_assignees(&state, auth).await },
    )
    .await
}
