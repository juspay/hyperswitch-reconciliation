use actix_web::{web, HttpRequest, Responder};
use api_models::{
    bulk_operations::{
        BulkOperationResultItem, StagingEntryBulkAction, StagingEntryBulkOperationRequest,
        StagingEntryBulkSelection,
    },
    staging_entry::{
        ApiListStagingEntriesFilters, ApiStagingEntryCoarseStatus, ApiStagingEntryListFiltersV2,
        ApiStagingEntryStatus, GetStagingEntryResponse, ListStagingEntriesV2Request,
        StagingEntryCreateRequest, StagingEntryCreateRequestV2, StagingEntryListV2Page,
        StagingEntryUpdateRequest, VoidStagingEntryRequest,
    },
};
use common_types::{
    date_time::TimeRange,
    pagination::{CursorDirection, SortOrder},
    permissions::Permission,
    staging_entry::{StagingEntryEffectiveAtSortField, StagingEntryListSortField},
};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::JwtAuth},
};

/// OpenAPI documentation for staging entry endpoints
#[derive(OpenApi)]
#[openapi(
    paths(
        create_staging_entry,
        create_staging_entry_with_validation,
        get_staging_entry_by_id,
        list_staging_entries_by_profile_id,
        list_staging_entries_v2,
        execute_staging_entry_bulk_operations,
        update_staging_entry,
        void_staging_entry,
    ),
    components(
        schemas(
            StagingEntryCreateRequest,
            StagingEntryCreateRequestV2,
            StagingEntryUpdateRequest,
            VoidStagingEntryRequest,
            GetStagingEntryResponse,
            ApiStagingEntryListFiltersV2,
            ListStagingEntriesV2Request,
            StagingEntryListSortField,
            StagingEntryEffectiveAtSortField,
            SortOrder,
            CursorDirection,
            StagingEntryListV2Page,
            common_types::amount::AmountResponse,
            common_enums::BalanceDirection,
            ApiStagingEntryStatus,
            ApiStagingEntryCoarseStatus,
            common_types::staging_entry::status::StagingEntryManualReviewData,
            common_enums::StagingEntryProcessingMode,
            common_enums::Currency,
            ApiListStagingEntriesFilters,
            TimeRange,
            StagingEntryBulkOperationRequest,
            StagingEntryBulkAction,
            StagingEntryBulkSelection,
            BulkOperationResultItem,
        )
    ),
    tags(
        (name = "Staging Entries", description = "Staging entry management endpoints"),
    ),
)]
pub struct StagingEntryApi;

#[utoipa::path(
    post,
    path = "/staging_entries",
    tag = "Staging Entries",
    request_body = StagingEntryCreateRequest,
    responses(
        (status = 201, description = "Staging entry created successfully", body = GetStagingEntryResponse),
        (status = 400, description = "Bad request - Invalid amount or validation error"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Account not found - Account does not exist or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn create_staging_entry(
    state: web::Data<AppState>,
    req: web::Json<StagingEntryCreateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::staging_entry::create_staging_entry(state, req.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/staging_entries/{id}",
    tag = "Staging Entries",
    params(
        ("id" = String, Path, description = "Staging entry primary ID")
    ),
    responses(
        (status = 200, description = "Staging entry found", body = GetStagingEntryResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Staging entry not found or account does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn get_staging_entry_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryRead),
        &http_req,
        state,
        |state, auth| async move {
            let id = path.into_inner();
            core::staging_entry::get_staging_entry_by_id(state, id, auth).await
        },
    )
    .await
}

/// List staging entries by profile ID
#[utoipa::path(
    get,
    path = "/staging_entries",
    tag = "Staging Entries",
    params(
        ApiListStagingEntriesFilters
    ),
    security(
        ("Bearer" = [])
    ),
    responses(
        (status = 200, description = "Staging entries retrieved successfully", body = Vec<GetStagingEntryResponse>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_staging_entries_by_profile_id(
    state: web::Data<AppState>,
    query: web::Query<ApiListStagingEntriesFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryRead),
        &http_req,
        state,
        |state, auth| async move {
            let filters = query.into_inner();
            core::staging_entry::list_staging_entries_by_profile_id(state, filters, auth).await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/staging_entries/v2/list",
    tag = "Staging Entries",
    request_body = ListStagingEntriesV2Request,
    security(("Bearer" = [])),
    responses(
        (status = 200, description = "Paginated staging entries", body = StagingEntryListV2Page),
        (status = 400, description = "Bad request - invalid cursor or filter validation error"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_staging_entries_v2(
    state: web::Data<AppState>,
    req: web::Json<ListStagingEntriesV2Request>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryRead),
        &http_req,
        state,
        |state, auth| async move {
            core::staging_entry::list_staging_entries_v2(state, req.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    post,
    path = "/staging_entries/bulk_operations",
    tag = "Staging Entries",
    request_body = StagingEntryBulkOperationRequest,
    responses(
        (status = 200, description = "Bulk operation processed", body = Vec<BulkOperationResultItem>),
        (status = 400, description = "Bad request - Invalid selection or action payload"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn execute_staging_entry_bulk_operations(
    state: web::Data<AppState>,
    req: web::Json<StagingEntryBulkOperationRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::bulk_operations::execute_staging_entry_bulk_operation(
                state,
                req.into_inner(),
                auth,
            )
            .await
        },
    )
    .await
}

#[utoipa::path(
    put,
    path = "/staging_entries/{id}",
    tag = "Staging Entries",
    request_body = StagingEntryUpdateRequest,
    params(
        ("id" = String, Path, description = "Staging entry primary ID")
    ),
    responses(
        (status = 200, description = "Staging entry updated successfully", body = GetStagingEntryResponse),
        (status = 400, description = "Bad request - Invalid amount or validation error"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Staging entry or account not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn update_staging_entry(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<StagingEntryUpdateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryWrite),
        &http_req,
        state,
        |state, auth| async move {
            let id = path.into_inner();
            core::staging_entry::update_staging_entry(state, id, req.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    put,
    path = "/staging_entries/{id}/void",
    tag = "Staging Entries",
    request_body = VoidStagingEntryRequest,
    params(
        ("id" = String, Path, description = "Staging entry primary ID")
    ),
    responses(
        (status = 200, description = "Staging entry voided successfully", body = GetStagingEntryResponse),
        (status = 400, description = "Bad request - Invalid request or validation error"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Staging entry not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn void_staging_entry(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<VoidStagingEntryRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryWrite),
        &http_req,
        state,
        |state, auth| async move {
            let id = path.into_inner();
            core::staging_entry::void_staging_entry(state, id, req.into_inner(), auth).await
        },
    )
    .await
}

/// Create staging entry with metadata schema validation (V2)
#[utoipa::path(
    post,
    path = "/staging_entries/v2",
    tag = "Staging Entries",
    request_body = StagingEntryCreateRequestV2,
    responses(
        (status = 201, description = "Staging entry created successfully with validation", body = GetStagingEntryResponse),
        (status = 400, description = "Bad request - Validation error or invalid metadata schema"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Account or metadata schema not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn create_staging_entry_with_validation(
    state: web::Data<AppState>,
    req: web::Json<StagingEntryCreateRequestV2>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconStagingEntryWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::staging_entry::create_staging_entry_with_validation(state, req.into_inner(), auth)
                .await
        },
    )
    .await
}
