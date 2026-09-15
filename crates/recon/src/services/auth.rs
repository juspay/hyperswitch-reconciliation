use actix_web::{http::header::HeaderMap, HttpRequest};
use common_types::permissions::Permission;
use common_utils::{auth::AuthContext, errors::ReportSwitchExt, id_type::MerchantId};
use error_stack::{report, Result, ResultExt};
use external_services::user::{
    interfaces::UserInterface, types::request::AuthorizeUserTokenRequest,
};
use http::header;
use jsonwebtoken::{decode, DecodingKey, Validation};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use super::option_ext::OptionExt;
use crate::{
    db::{business_profile::BusinessProfileInterface, merchant_account::MerchantAccountInterface},
    errors::{conversions::StorageErrorExt, ReconAppError},
    routes::AppState,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub exp: u64,
}

pub fn extract_token_from_header(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|token| token.to_string())
}

/// Trait for different authentication validators
pub trait AuthValidator {
    type AuthData: Send;

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError>;
}

/// No authentication required - returns unit type
pub struct NoAuth;

impl AuthValidator for NoAuth {
    type AuthData = ();

    async fn validate(
        &self,
        _req: &HttpRequest,
        _state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct AdminApiAuth;

impl AuthValidator for AdminApiAuth {
    type AuthData = ();

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        let request_admin_api_key =
            get_api_key(req.headers()).change_context(ReconAppError::Unauthorized)?;
        let admin_api_key = &state.config.auth.get_inner().admin_api_key;

        if request_admin_api_key != admin_api_key.peek() {
            Err(report!(ReconAppError::Unauthorized).attach_printable("Admin Auth Failure"))?;
        }

        Ok(())
    }
}

/// Configurable JWT authentication validator
pub struct JwtAuth {
    merchant_id: Option<MerchantId>,
    profile_id: Option<String>,
    permission: Option<Permission>,
}

impl JwtAuth {
    /// Create basic JWT auth with permission (no additional field validation)
    pub fn basic(permission: Permission) -> Self {
        Self {
            merchant_id: None,
            profile_id: None,
            permission: Some(permission),
        }
    }

    /// Create basic JWT auth without permission (no additional field validation)
    pub fn basic_without_permission() -> Self {
        Self {
            merchant_id: None,
            profile_id: None,
            permission: None,
        }
    }

    /// Create JWT auth that validates merchant_id matches the provided value
    pub fn with_merchant_without_permission(merchant_id: MerchantId) -> Self {
        Self {
            merchant_id: Some(merchant_id),
            profile_id: None,
            permission: None,
        }
    }

    /// Create JWT auth that validates profile_id matches the provided value
    pub fn with_profile_without_permission(profile_id: String) -> Self {
        Self {
            merchant_id: None,
            profile_id: Some(profile_id),
            permission: None,
        }
    }
}

impl AuthValidator for JwtAuth {
    type AuthData = AuthContext;

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        let token =
            extract_token_from_header(req).ok_or_else(|| report!(ReconAppError::Unauthorized))?;

        let decoding_key =
            DecodingKey::from_secret(state.config.auth.get_inner().jwt_secret.peek().as_ref());
        let claims = decode::<Claims>(&token, &decoding_key, &Validation::default())
            .map(|data| data.claims)
            .change_context(ReconAppError::InvalidToken)?;

        let user_id = claims.user_id;
        let merchant_id = claims.merchant_id;
        let profile_id = claims.profile_id;

        if user_id.is_empty() {
            return Err(report!(ReconAppError::Unauthorized)
                .attach_printable("Missing user_id in JWT claims"));
        }

        if let Some(expected_merchant_id) = &self.merchant_id {
            if merchant_id != *expected_merchant_id {
                return Err(report!(ReconAppError::Unauthorized));
            }
        }

        if let Some(expected_profile_id) = &self.profile_id {
            if profile_id != *expected_profile_id {
                return Err(report!(ReconAppError::Unauthorized));
            }
        }

        state
            .store
            .find_merchant_account_by_id(&merchant_id)
            .await
            .to_not_found_response(|| ReconAppError::Unauthorized)?;

        let business_profile = state
            .store
            .find_business_profile_by_id(&profile_id)
            .await
            .to_not_found_response(|| ReconAppError::Unauthorized)?;

        if business_profile.merchant_id != merchant_id {
            return Err(report!(ReconAppError::Unauthorized));
        }

        if let Some(permission) = self.permission {
            state
                .user_client
                .authorize_user_token(
                    merchant_id.clone(),
                    profile_id.clone(),
                    AuthorizeUserTokenRequest {
                        token: Secret::new(token),
                        permission,
                    },
                )
                .await
                .switch()?;
        }

        Ok(AuthContext {
            user_id,
            merchant_id,
            profile_id,
        })
    }
}

pub fn extract_api_key_from_header(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

pub fn get_api_key(headers: &HeaderMap) -> Result<&str, ReconAppError> {
    get_header_value_by_key("api-key".into(), headers)?.get_required_value("api_key")
}

pub fn get_header_value_by_key(
    key: String,
    headers: &HeaderMap,
) -> Result<Option<&str>, ReconAppError> {
    headers
        .get(&key)
        .map(|source_str| {
            source_str
                .to_str()
                .change_context(ReconAppError::InternalServerError)
                .attach_printable(format!(
                    "Failed to convert header value to string for header key: {key}"
                ))
        })
        .transpose()
}
