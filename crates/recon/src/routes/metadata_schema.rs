use actix_web::{web, HttpRequest, Responder};
use api_models::metadata_schema::{
    AmountSchemaConfig, BalanceDirectionSchemaConfig, CreateMetadataSchemaRequest,
    CurrencySchemaConfig, EffectiveAtSchemaConfig, FieldType, FieldsConfig,
    ListMetadataSchemasResponse, MetadataFieldSchemaConfig, MetadataSchemaData,
    MetadataSchemaResponse, MetadataSchemaUpdateRequest, NumberValidationRule,
    StringValidationRule, UniqueConstraint, UniqueConstraintType,
};
use common_types::{metadata_schema::ListMetadataSchemasFilters, permissions::Permission};
use utoipa::OpenApi;

use super::AppState;
use crate::{
    core,
    services::{api, auth::JwtAuth},
};

/// OpenAPI documentation for metadata schema endpoints
#[derive(OpenApi)]
#[openapi(
    paths(
        create_metadata_schema,
        get_metadata_schema_by_id,
        list_metadata_schemas,
        update_metadata_schema
    ),
    components(
        schemas(
            CreateMetadataSchemaRequest,
            MetadataSchemaUpdateRequest,
            MetadataSchemaResponse,
            ListMetadataSchemasResponse,
            MetadataSchemaData,
            FieldsConfig,
            MetadataFieldSchemaConfig,
            CurrencySchemaConfig,
            AmountSchemaConfig,
            EffectiveAtSchemaConfig,
            BalanceDirectionSchemaConfig,
            FieldType,
            StringValidationRule,
            NumberValidationRule,
            UniqueConstraint,
            UniqueConstraintType,
            ListMetadataSchemasFilters,
        )
    ),
    tags(
        (name = "Metadata Schemas", description = "Metadata schema management endpoints"),
    ),
)]
pub struct MetadataSchemaApi;

#[utoipa::path(
    post,
    path = "/metadata_schemas",
    tag = "Metadata Schemas",
    request_body = CreateMetadataSchemaRequest,
    responses(
        (status = 201, description = "Metadata schema created successfully", body = MetadataSchemaResponse),
        (status = 400, description = "Bad request - Invalid schema data"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 403, description = "Forbidden - Profile ID mismatch"),
        (status = 404, description = "Account not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn create_metadata_schema(
    state: web::Data<AppState>,
    req: web::Json<CreateMetadataSchemaRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::metadata_schema::create_metadata_schema(state, req.into_inner(), auth).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/metadata_schemas/{schema_id}",
    tag = "Metadata Schemas",
    params(
        ("schema_id" = String, Path, description = "Metadata schema ID")
    ),
    responses(
        (status = 200, description = "Metadata schema found", body = MetadataSchemaResponse),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Metadata schema not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn get_metadata_schema_by_id(
    state: web::Data<AppState>,
    path: web::Path<String>,
    http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationRead),
        &http_req,
        state,
        |state, auth| async move {
            core::metadata_schema::get_metadata_schema_by_id(state, id, auth).await
        },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/metadata_schemas",
    tag = "Metadata Schemas",
    params(
        ListMetadataSchemasFilters
    ),
    responses(
        (status = 200, description = "List of metadata schemas", body = ListMetadataSchemasResponse),
        (status = 400, description = "Bad request - Invalid filter parameters"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn list_metadata_schemas(
    state: web::Data<AppState>,
    query: web::Query<ListMetadataSchemasFilters>,
    http_req: HttpRequest,
) -> impl Responder {
    let filters = query.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationRead),
        &http_req,
        state,
        |state, auth| async move {
            core::metadata_schema::list_metadata_schemas(state, auth, filters).await
        },
    )
    .await
}

#[utoipa::path(
    put,
    path = "/metadata_schemas/{schema_id}",
    tag = "Metadata Schemas",
    params(
        ("schema_id" = String, Path, description = "Metadata schema ID")
    ),
    request_body = MetadataSchemaUpdateRequest,
    responses(
        (status = 200, description = "Metadata schema updated successfully (new version created)", body = MetadataSchemaResponse),
        (status = 400, description = "Bad request - Invalid schema data"),
        (status = 401, description = "Unauthorized - Invalid or missing JWT token"),
        (status = 404, description = "Metadata schema not found or does not belong to authenticated user's profile"),
        (status = 500, description = "Internal server error"),
    ),
    security(("Bearer" = []))
)]
pub async fn update_metadata_schema(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<MetadataSchemaUpdateRequest>,
    http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    api::server_wrap(
        JwtAuth::basic(Permission::ProfileReconTransformationWrite),
        &http_req,
        state,
        |state, auth| async move {
            core::metadata_schema::update_metadata_schema(state, id, req.into_inner(), auth).await
        },
    )
    .await
}
