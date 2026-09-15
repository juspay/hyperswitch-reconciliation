use actix_web::{web, HttpRequest, Responder};
use api_models::{
    metadata_schema::MetadataSchemaResponseV2,
    transformation::{
        CreateTransformationConfigV2Request, TransformationConfigListResponse,
        TransformationConfigResponse, TransformationHistoryListResponse,
        TransformationHistoryResponse, TransformationReportFormat, TransformationTriggerRequest,
        UpdateTransformationConfigRequest,
    },
};
use common_types::{
    permissions::Permission,
    transformation::{ListTransformationConfigsFilters, ListTransformationHistoryFilters},
};
use utoipa::OpenApi;

use crate::{
    core,
    routes::AppState,
    services::{
        api,
        auth::{JwtAuth, NoAuth},
    },
};

/// Execute transformation for an ingestion
#[utoipa::path(
    post,
    path = "/transformations/{ingestion_id}/{ingestion_history_id}/trigger",
    params(
        ("ingestion_id" = String, Path, description = "Ingestion ID"),
        ("ingestion_history_id" = String, Path, description = "Ingestion History ID")
    ),
    request_body = TransformationTriggerRequest,
    responses(
        (status = 200, description = "Transformation executed successfully"),
        (status = 404, description = "Ingestion or transformation config not found"),
        (status = 422, description = "Transformation failed"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "Execute Transformation"
)]
pub async fn execute_transformation(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
    req: web::Json<TransformationTriggerRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let (ingestion_id, ingestion_history_id) = path.into_inner();
    let payload = req.into_inner();

    api::server_wrap(NoAuth, &http_req, state, |state, _auth| async move {
        core::transformation::execute_transformation(
            state,
            ingestion_id,
            ingestion_history_id,
            payload,
        )
        .await
    })
    .await
}

/// Get transformation history with config by ID
#[utoipa::path(
    get,
    path = "/transformations/history/{transformation_history_id}",
    params(
        ("transformation_history_id" = String, Path, description = "Transformation History ID")
    ),
    responses(
        (status = 200, description = "Transformation history with config retrieved successfully", body = TransformationHistoryResponse),
        (status = 404, description = "Transformation history not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "Get Transformation History with Config",
    security(("Bearer" = []))
)]
pub async fn find_transformation_history_with_config_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let transformation_history_id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationRead),
        &http_req,
        state,
        |state, _auth| async move {
            core::transformation::find_transformation_history_with_config_by_id(
                &state,
                &transformation_history_id,
            )
            .await
        },
    )
    .await
}

/// Download a generated transformation report
#[utoipa::path(
    get,
    path = "/transformations/history/{transformation_history_id}/reports/{report_format}",
    params(
        ("transformation_history_id" = String, Path, description = "Transformation History ID"),
        ("report_format" = TransformationReportFormat, Path, description = "Report format: csv or json")
    ),
    responses(
        (status = 200, description = "Transformation report downloaded successfully", content_type = "application/octet-stream"),
        (status = 404, description = "Transformation history or report not found"),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "Download Transformation Report",
    security(("Bearer" = []))
)]
pub async fn download_transformation_report(
    state: web::Data<AppState>,
    path: web::Path<(String, TransformationReportFormat)>,
    http_req: HttpRequest,
) -> impl Responder {
    let (transformation_history_id, report_format) = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationRead),
        &http_req,
        state,
        |state, auth| async move {
            core::transformation::download_transformation_report(
                &state,
                &transformation_history_id,
                report_format,
                &auth.profile_id,
            )
            .await
        },
    )
    .await
}

/// List transformation history with config by profile ID with filters
#[utoipa::path(
    get,
    path = "/transformations/history",
    params(
        ListTransformationHistoryFilters
    ),
    responses(
        (status = 200, description = "Transformation history with config list retrieved successfully", body = TransformationHistoryListResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "List Transformation History with Config by Profile",
    security(("Bearer" = []))
)]
pub async fn list_transformation_history_with_config_by_profile_id(
    state: web::Data<AppState>,
    query: web::Query<ListTransformationHistoryFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationRead),
        &http_req,
        state,
        |state, auth| async move {
            let filters = query.into_inner();
            core::transformation::list_transformation_history_with_config_by_profile_id(
                &state,
                auth.profile_id,
                filters,
            )
            .await
        },
    )
    .await
}

/// List transformation configs by profile ID with filters
#[utoipa::path(
    get,
    path = "/transformations/configs",
    params(
        ListTransformationConfigsFilters
    ),
    responses(
        (status = 200, description = "Transformation configs retrieved successfully", body = TransformationConfigListResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "List Transformation Configs by Profile",
    security(("Bearer" = []))
)]
pub async fn list_transformation_configs_by_profile_id(
    state: web::Data<AppState>,
    query: web::Query<ListTransformationConfigsFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationRead),
        &http_req,
        state,
        |state, auth| async move {
            let filters = query.into_inner();
            core::transformation::list_transformation_configs_by_profile_id(state, filters, auth)
                .await
        },
    )
    .await
}
/// Create V2 transformation config with metadata schema
#[utoipa::path(
    post,
    path = "/transformations/v2/configs",
    request_body = CreateTransformationConfigV2Request,
    responses(
        (status = 201, description = "V2 transformation config created successfully", body = TransformationConfigResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized - invalid or missing Bearer token"),
        (status = 404, description = "Metadata schema not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "Create Transformation Config V2",
    security(("Bearer" = []))
)]
pub async fn create_transformation_config_v2(
    state: web::Data<AppState>,
    req: web::Json<CreateTransformationConfigV2Request>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationWrite),
        &http_req,
        state,
        |state, auth| async move {
            let request = req.into_inner();
            core::transformation::create_transformation_config_v2(state, request, auth).await
        },
    )
    .await
}

/// Get transformation config by ID
#[utoipa::path(
    get,
    path = "/transformations/configs/{transformation_id}",
    params(
        ("transformation_id" = String, Path, description = "Transformation Config ID")
    ),
    responses(
        (status = 200, description = "Transformation config retrieved successfully", body = TransformationConfigResponse),
        (status = 404, description = "Transformation config not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "Get Transformation Config by ID",
    security(("Bearer" = []))
)]
pub async fn get_transformation_config_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let transformation_id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationRead),
        &http_req,
        state,
        |state, auth| async move {
            core::transformation::get_transformation_config_by_id(state, transformation_id, auth)
                .await
        },
    )
    .await
}

/// Update transformation config by ID
#[utoipa::path(
    put,
    path = "/transformations/configs/{transformation_id}",
    params(
        ("transformation_id" = String, Path, description = "Transformation Config ID")
    ),
    request_body = UpdateTransformationConfigRequest,
    responses(
        (status = 200, description = "Transformation config updated successfully", body = TransformationConfigResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized - invalid or missing Bearer token"),
        (status = 404, description = "Transformation config not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "Update Transformation Config by ID",
    security(("Bearer" = []))
)]
pub async fn update_transformation_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<UpdateTransformationConfigRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let transformation_id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationWrite),
        &http_req,
        state,
        |state, auth| async move {
            let request = req.into_inner();
            core::transformation::update_transformation_config(
                state,
                transformation_id,
                request,
                auth,
            )
            .await
        },
    )
    .await
}

/// Get Metadata by transformation config ID
#[utoipa::path(
    get,
    path = "/transformations/configs/{transformation_id}/metadata",
    params(
        ("transformation_id" = String, Path, description = "Transformation Config ID")
    ),
    responses(
        (status = 200, description = "Metadata retrieved successfully", body = MetadataSchemaResponseV2),
        (status = 404, description = "Metadata Schema not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Transformations",
    operation_id = "Get Metadata by Transformation Config ID",
    security(("Bearer" = []))
)]
pub async fn get_metadata_schema_by_transformation_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let transformation_id = path.into_inner();

    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationRead),
        &http_req,
        state,
        |state, auth| async move {
            core::transformation::get_metadata_schema_by_transformation_id(
                state,
                transformation_id,
                auth,
            )
            .await
        },
    )
    .await
}

#[derive(OpenApi)]
#[openapi(
    paths(
        execute_transformation,
        find_transformation_history_with_config_by_id,
        download_transformation_report,
        list_transformation_history_with_config_by_profile_id,
        list_transformation_configs_by_profile_id,
        create_transformation_config_v2,
        get_transformation_config_by_id,
        update_transformation_config,
        get_metadata_schema_by_transformation_id
    ),
    components(schemas(
        TransformationTriggerRequest,
        TransformationReportFormat,
        TransformationHistoryResponse,
        TransformationHistoryListResponse,
        TransformationConfigResponse,
        TransformationConfigListResponse,
        CreateTransformationConfigV2Request,
        UpdateTransformationConfigRequest,
        ListTransformationHistoryFilters,
        ListTransformationConfigsFilters,
        MetadataSchemaResponseV2
    )),
    tags((name = "Transformations", description = "Transform ingested data using configurable rules"))
)]
pub struct TransformationApi;
