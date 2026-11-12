use api_models::api_key::{
    ApiKeyResponse as ApiKeyApiResponse, CreateApiKeyRequest, CreateApiKeyResponse,
    ListApiKeysResponse, UpdateApiKeyRequest, UpdateApiKeyType,
};
use common_utils::{api_key::PlaintextApiKey, auth::AuthContext, date_time};
use diesel_models::api_key::{ApiKeyNew, ApiKeyUpdate};
use error_stack::ResultExt;
use masking::StrongSecret;
use storage_impl::api_key::ApiKeyInterface;

use crate::{
    env,
    errors::{
        api_key::{ApiKeyError, ApiKeyResponse},
        conversions::StorageErrorExt,
    },
    routes::AppState,
    types::ApplicationResponse,
    utils::api_key::{ApiKeyExt, ApiKeyRequestValidator},
};

pub async fn create_api_key(
    state: AppState,
    request: CreateApiKeyRequest,
    auth_context: AuthContext,
) -> ApiKeyResponse<CreateApiKeyResponse> {
    let now = date_time::now();
    request.validate(now)?;

    let plaintext_key = PlaintextApiKey::generate(env::which().prefix());

    let api_key = state
        .store
        .insert_api_key(ApiKeyNew::new(
            auth_context.merchant_id,
            request.name,
            request.description,
            plaintext_key.hash(&state.api_key_hash_key).into(),
            plaintext_key.display_prefix(),
            Some(auth_context.user_id),
            request.expires_at,
            now,
        ))
        .await
        .change_context(ApiKeyError::InternalServerError)?;

    Ok(ApplicationResponse::json_created_no_store(
        CreateApiKeyResponse {
            api_key_details: api_key.to_api_response(),
            api_key: StrongSecret::new(plaintext_key.expose().to_string()),
        },
    ))
}

pub async fn list_api_keys(
    state: AppState,
    auth_context: AuthContext,
) -> ApiKeyResponse<ListApiKeysResponse> {
    let api_keys = state
        .store
        .list_active_api_keys_by_merchant_id(&auth_context.merchant_id)
        .await
        .change_context(ApiKeyError::InternalServerError)?;

    Ok(ApplicationResponse::json(ListApiKeysResponse {
        api_keys: api_keys
            .into_iter()
            .map(ApiKeyExt::to_api_response)
            .collect(),
    }))
}

pub async fn get_api_key(
    state: AppState,
    api_key_id: &str,
    auth_context: AuthContext,
) -> ApiKeyResponse<ApiKeyApiResponse> {
    let api_key = state
        .store
        .find_active_api_key_by_merchant_id_and_id(&auth_context.merchant_id, api_key_id)
        .await
        .to_not_found_response(|| ApiKeyError::ApiKeyNotFound)?;

    Ok(ApplicationResponse::json(api_key.to_api_response()))
}

pub async fn update_api_key(
    state: AppState,
    api_key_id: &str,
    request: UpdateApiKeyRequest,
    auth_context: AuthContext,
) -> ApiKeyResponse<ApiKeyApiResponse> {
    request.validate(date_time::now())?;

    let update = match request.update_type {
        UpdateApiKeyType::RevokeUpdate {} => ApiKeyUpdate::RevokeUpdate,
        UpdateApiKeyType::DetailsUpdate { name, description } => {
            ApiKeyUpdate::DetailsUpdate { name, description }
        }
    };

    let api_key = state
        .store
        .update_api_key(&auth_context.merchant_id, api_key_id, update)
        .await
        .to_not_found_response(|| ApiKeyError::ApiKeyNotFound)?;

    Ok(ApplicationResponse::json(api_key.to_api_response()))
}
