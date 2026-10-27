use api_models::metadata_schema::{
    CreateMetadataSchemaRequest, ListMetadataSchemasResponse, MetadataSchemaResponse,
    MetadataSchemaUpdateRequest,
};
use common_types::metadata_schema::ListMetadataSchemasFilters;
use common_utils::{auth::AuthContext, errors::ReportSwitchExt};
use diesel_models::metadata_schema::{MetadataSchemaNew, MetadataSchemaVersionUpdate};
use error_stack::ResultExt;
use storage_impl::{account::AccountInterface, metadata_schema::MetadataSchemaInterface};

use crate::{
    domain::metadata_schema::ConfigValidator,
    errors::{
        conversions::StorageErrorExt,
        metadata_schema::{MetadataSchemaError, MetadataSchemaResult},
    },
    routes::AppState,
    types::ApplicationResponse,
    utils::metadata_schema::{MetadataSchemaDataApiExt, MetadataSchemaDataDieselExt},
};

/// Create a new metadata schema
pub async fn create_metadata_schema(
    state: AppState,
    request: CreateMetadataSchemaRequest,
    auth_context: AuthContext,
) -> MetadataSchemaResult<MetadataSchemaResponse> {
    // Validate that the account belongs to the authenticated user's profile
    state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &request.account_id)
        .await
        .to_not_found_response(|| MetadataSchemaError::AccountNotFound {
            account_id: request.account_id.clone(),
        })?;

    // Convert API schema data to diesel schema data
    let diesel_schema_data = request.schema_data.to_diesel_schema();

    diesel_schema_data.validate_config().switch()?;

    // Create new metadata schema
    let new_schema = MetadataSchemaNew::new(
        auth_context.profile_id,
        request.account_id,
        diesel_schema_data,
    );

    // Insert into database
    let metadata_schema = state
        .store
        .insert_metadata_schema(new_schema)
        .await
        .change_context(MetadataSchemaError::InternalServerError)?;

    // Convert to API response
    let response = MetadataSchemaResponse {
        id: metadata_schema.id,
        schema_id: metadata_schema.schema_id,
        profile_id: metadata_schema.profile_id,
        account_id: metadata_schema.account_id,
        schema_data: metadata_schema.schema_data.to_api_response(),
        version: metadata_schema.version,
        created_at: metadata_schema.created_at,
        last_modified_at: metadata_schema.last_modified_at,
    };

    Ok(ApplicationResponse::json_created(response))
}

/// Get metadata schema by ID
pub async fn get_metadata_schema_by_id(
    state: AppState,
    id: String,
    auth_context: AuthContext,
) -> MetadataSchemaResult<MetadataSchemaResponse> {
    // Find metadata schema by ID and profile
    let metadata_schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(&id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| MetadataSchemaError::MetadataSchemaNotFound)?;

    // Convert to API response
    let response = MetadataSchemaResponse {
        id: metadata_schema.id,
        schema_id: metadata_schema.schema_id,
        profile_id: metadata_schema.profile_id,
        account_id: metadata_schema.account_id,
        schema_data: metadata_schema.schema_data.to_api_response(),
        version: metadata_schema.version,
        created_at: metadata_schema.created_at,
        last_modified_at: metadata_schema.last_modified_at,
    };

    Ok(ApplicationResponse::json(response))
}

/// List metadata schemas for the authenticated user's profile
pub async fn list_metadata_schemas(
    state: AppState,
    auth_context: AuthContext,
    filters: ListMetadataSchemasFilters,
) -> MetadataSchemaResult<ListMetadataSchemasResponse> {
    // Validate filters
    filters
        .validate()
        .map_err(|_| MetadataSchemaError::TimeRangeValidationError)?;

    // Get schemas for the authenticated user's profile with filters
    let metadata_schemas = state
        .store
        .list_metadata_schemas_by_profile_id(&auth_context.profile_id, filters)
        .await
        .change_context(MetadataSchemaError::InternalServerError)?;

    // Convert to API response
    let response: ListMetadataSchemasResponse = metadata_schemas
        .into_iter()
        .map(|schema| MetadataSchemaResponse {
            id: schema.id,
            schema_id: schema.schema_id,
            profile_id: schema.profile_id,
            account_id: schema.account_id,
            schema_data: schema.schema_data.to_api_response(),
            version: schema.version,
            created_at: schema.created_at,
            last_modified_at: schema.last_modified_at,
        })
        .collect();

    Ok(ApplicationResponse::json(response))
}

/// Update metadata schema (creates a new version)
pub async fn update_metadata_schema(
    state: AppState,
    id: String,
    request: MetadataSchemaUpdateRequest,
    auth_context: AuthContext,
) -> MetadataSchemaResult<MetadataSchemaResponse> {
    // Find the current metadata schema by ID and profile
    let current_schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(&id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| MetadataSchemaError::MetadataSchemaNotFound)?;

    // Convert API schema data to diesel schema data and create version update
    let diesel_schema_data = request.schema_data.to_diesel_schema();
    let version_update = MetadataSchemaVersionUpdate::SchemaUpdate {
        schema_data: diesel_schema_data,
    };

    let updated_schema = state
        .store
        .create_new_metadata_schema_version(current_schema, version_update)
        .await
        .change_context(MetadataSchemaError::InternalServerError)?;

    // Convert to API response
    let response = MetadataSchemaResponse {
        id: updated_schema.id,
        schema_id: updated_schema.schema_id,
        profile_id: updated_schema.profile_id,
        account_id: updated_schema.account_id,
        schema_data: updated_schema.schema_data.to_api_response(),
        version: updated_schema.version,
        created_at: updated_schema.created_at,
        last_modified_at: updated_schema.last_modified_at,
    };

    Ok(ApplicationResponse::json(response))
}
