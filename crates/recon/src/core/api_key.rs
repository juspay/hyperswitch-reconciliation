use api_models::api_key::{
    ApiKeyResponse, CreateApiKeyRequest, CreateApiKeyResponse, ListApiKeysResponse,
    UpdateApiKeyRequest,
};
use common_enums::ApiKeyStatus;
use common_utils::{
    api_key::PlaintextApiKey, date_time, id_generation::generate_id_with_default_len,
};
use diesel_models::api_key::{ApiKey, ApiKeyNew, ApiKeyUpdate};
use error_stack::{report, ResultExt};
use masking::Secret;
use storage_impl::api_key::ApiKeyInterface;
use time::PrimitiveDateTime;

use crate::{
    env,
    errors::{
        api_key::{ApiKeyError, ApiKeyResult},
        conversions::StorageErrorExt,
    },
    routes::AppState,
    services::auth::ManagementAuthContext,
    types::ApplicationResponse,
};

const MAX_NAME_LENGTH: usize = 64;
const MAX_DESCRIPTION_LENGTH: usize = 256;

fn invalid_request<T>(message: &str) -> Result<T, error_stack::Report<ApiKeyError>> {
    Err(report!(ApiKeyError::InvalidRequest {
        message: message.to_string(),
    }))
}

fn validate_name(name: &str) -> Result<(), error_stack::Report<ApiKeyError>> {
    if name.trim().is_empty() {
        return invalid_request("name must not be empty");
    }
    if name.chars().count() > MAX_NAME_LENGTH {
        return invalid_request(&format!(
            "name must be at most {MAX_NAME_LENGTH} characters"
        ));
    }
    Ok(())
}

fn validate_description(description: &str) -> Result<(), error_stack::Report<ApiKeyError>> {
    if description.chars().count() > MAX_DESCRIPTION_LENGTH {
        return invalid_request(&format!(
            "description must be at most {MAX_DESCRIPTION_LENGTH} characters"
        ));
    }
    Ok(())
}

fn to_response(api_key: ApiKey) -> ApiKeyResponse {
    ApiKeyResponse {
        id: api_key.id,
        merchant_id: api_key.merchant_id,
        name: api_key.name,
        description: api_key.description,
        prefix: api_key.prefix,
        created_by: api_key.created_by,
        status: api_key.status,
        created_at: api_key.created_at,
        expires_at: api_key.expires_at.into(),
        last_modified_at: api_key.last_modified_at,
    }
}

pub async fn create_api_key(
    state: AppState,
    request: CreateApiKeyRequest,
    auth_context: ManagementAuthContext,
) -> ApiKeyResult<CreateApiKeyResponse> {
    validate_name(&request.name)?;
    if let Some(description) = &request.description {
        validate_description(description)?;
    }

    let now = date_time::now();
    let expires_at: Option<PrimitiveDateTime> = request.expires_at.into();
    if expires_at.is_some_and(|expires_at| expires_at <= now) {
        return invalid_request("expires_at must be in the future");
    }

    let plaintext_key = PlaintextApiKey::generate(env::prefix_for_env());

    let api_key = state
        .store
        .insert_api_key(ApiKeyNew {
            id: generate_id_with_default_len("apikey"),
            merchant_id: auth_context.merchant_id,
            name: request.name.trim().to_string(),
            description: request.description,
            hashed_api_key: plaintext_key.hash(&state.api_key_hash_key).into(),
            prefix: plaintext_key.display_prefix(),
            created_by: auth_context.user_id,
            created_at: now,
            expires_at,
            last_modified_at: now,
            status: ApiKeyStatus::Active,
        })
        .await
        .change_context(ApiKeyError::InternalServerError)?;

    Ok(ApplicationResponse::json_created_no_store(
        CreateApiKeyResponse {
            api_key_details: to_response(api_key),
            api_key: Secret::new(plaintext_key.expose().to_string()),
        },
    ))
}

pub async fn list_api_keys(
    state: AppState,
    auth_context: ManagementAuthContext,
) -> ApiKeyResult<ListApiKeysResponse> {
    let api_keys = state
        .store
        .list_active_api_keys_by_merchant_id(&auth_context.merchant_id)
        .await
        .change_context(ApiKeyError::InternalServerError)?;

    Ok(ApplicationResponse::json(ListApiKeysResponse {
        api_keys: api_keys.into_iter().map(to_response).collect(),
    }))
}

pub async fn get_api_key(
    state: AppState,
    api_key_id: &str,
    auth_context: ManagementAuthContext,
) -> ApiKeyResult<ApiKeyResponse> {
    let api_key = state
        .store
        .find_active_api_key_by_merchant_id_and_id(&auth_context.merchant_id, api_key_id)
        .await
        .to_not_found_response(|| ApiKeyError::ApiKeyNotFound)?;

    Ok(ApplicationResponse::json(to_response(api_key)))
}

pub async fn update_api_key(
    state: AppState,
    api_key_id: &str,
    request: UpdateApiKeyRequest,
    auth_context: ManagementAuthContext,
) -> ApiKeyResult<ApiKeyResponse> {
    if let Some(name) = &request.name {
        validate_name(name)?;
    }
    if let Some(description) = &request.description {
        validate_description(description)?;
    }

    let api_key = state
        .store
        .update_api_key(
            &auth_context.merchant_id,
            api_key_id,
            ApiKeyUpdate::Update {
                name: request.name.map(|name| name.trim().to_string()),
                description: request.description,
            },
        )
        .await
        .to_not_found_response(|| ApiKeyError::ApiKeyNotFound)?;

    Ok(ApplicationResponse::json(to_response(api_key)))
}

pub async fn revoke_api_key(
    state: AppState,
    api_key_id: &str,
    auth_context: ManagementAuthContext,
) -> ApiKeyResult<ApiKeyResponse> {
    let api_key = state
        .store
        .update_api_key(&auth_context.merchant_id, api_key_id, ApiKeyUpdate::Revoke)
        .await
        .to_not_found_response(|| ApiKeyError::ApiKeyNotFound)?;

    Ok(ApplicationResponse::json(to_response(api_key)))
}
