use api_models::api_key::{
    ApiKeyResponse as ApiKeyApiResponse, CreateApiKeyRequest, CreateApiKeyResponse,
    ListApiKeysResponse, UpdateApiKeyRequest,
};
use common_enums::ApiKeyStatus;
use common_utils::{
    api_key::PlaintextApiKey, auth::AuthContext, date_time,
    id_generation::generate_id_with_default_len,
};
use diesel_models::api_key::{ApiKeyNew, ApiKeyUpdate};
use error_stack::ResultExt;
use masking::Secret;
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

    let plaintext_key = PlaintextApiKey::generate(env::prefix_for_env());

    let api_key = state
        .store
        .insert_api_key(ApiKeyNew {
            id: generate_id_with_default_len("apikey"),
            merchant_id: auth_context.merchant_id,
            name: request.name,
            description: request.description,
            hashed_api_key: plaintext_key.hash(&state.api_key_hash_key).into(),
            prefix: plaintext_key.display_prefix(),
            created_by: Some(auth_context.user_id),
            created_at: now,
            expires_at: request.expires_at,
            last_modified_at: now,
            status: ApiKeyStatus::Active,
        })
        .await
        .change_context(ApiKeyError::InternalServerError)?;

    Ok(ApplicationResponse::json_created_no_store(
        CreateApiKeyResponse {
            api_key_details: api_key.to_api_response(),
            api_key: Secret::new(plaintext_key.expose().to_string()),
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

    let api_key = state
        .store
        .update_api_key(
            &auth_context.merchant_id,
            api_key_id,
            ApiKeyUpdate::DetailsUpdate {
                name: request.name,
                description: request.description,
                status: request.status,
            },
        )
        .await
        .to_not_found_response(|| ApiKeyError::ApiKeyNotFound)?;

    Ok(ApplicationResponse::json(api_key.to_api_response()))
}
