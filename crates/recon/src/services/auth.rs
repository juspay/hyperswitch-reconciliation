use actix_web::{http::header::HeaderMap, HttpRequest};
use common_types::permissions::Permission;
use common_utils::{
    api_key::PlaintextApiKey,
    auth::{AuthContext, AuthMethod},
    date_time,
    errors::ReportSwitchExt,
    id_type::MerchantId,
};
use error_stack::{report, Result, ResultExt};
use external_services::user::{
    interfaces::UserInterface, types::request::AuthorizeUserTokenRequest,
};
use http::header;
use jsonwebtoken::{decode, DecodingKey, Validation};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use storage_impl::{
    api_key::ApiKeyInterface, business_profile::BusinessProfileInterface,
    merchant_account::MerchantAccountInterface,
};

use super::option_ext::OptionExt;

use crate::{
    errors::{conversions::StorageErrorExt, ReconAppError},
    routes::AppState,
};

pub mod auth_headers {
    pub const X_INTERNAL_API_KEY: &str = "x-internal-api-key";
    pub const X_MERCHANT_ID: &str = "x-merchant-id";
    pub const X_PROFILE_ID: &str = "x-profile-id";
}

const API_KEY_PREFIX: &str = "api-key=";
const ADMIN_API_KEY_PREFIX: &str = "admin-api-key=";

#[derive(Debug)]
pub enum Credential {
    /// The token itself is read again by `JwtAuth`.
    Bearer,
    ApiKey(Secret<String>),
    AdminApiKey(Secret<String>),
}

fn parse_secret_value(value: &str) -> Result<Secret<String>, ReconAppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(report!(ReconAppError::Unauthorized).attach_printable("Malformed credential"));
    }
    Ok(Secret::new(value.to_string()))
}

fn parse_authorization(value: &str) -> Result<Credential, ReconAppError> {
    let value = value.trim();
    if value.starts_with("Bearer ") {
        Ok(Credential::Bearer)
    } else if let Some(key) = value.strip_prefix(ADMIN_API_KEY_PREFIX) {
        parse_secret_value(key).map(Credential::AdminApiKey)
    } else if let Some(key) = value.strip_prefix(API_KEY_PREFIX) {
        parse_secret_value(key).map(Credential::ApiKey)
    } else {
        Err(report!(ReconAppError::Unauthorized).attach_printable("Unknown credential scheme"))
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, ReconAppError> {
    let mut values = headers.get_all(name);
    let (Some(value), None) = (values.next(), values.next()) else {
        // Two headers are two credentials.
        return match headers.get(name) {
            Some(_) => Err(report!(ReconAppError::Unauthorized)
                .attach_printable(format!("More than one {name} header"))),
            None => Ok(None),
        };
    };

    value.to_str().map(Some).map_err(|_| {
        report!(ReconAppError::Unauthorized).attach_printable(format!("Unreadable {name} header"))
    })
}

pub fn extract_credential(headers: &HeaderMap) -> Result<Credential, ReconAppError> {
    match header_str(headers, header::AUTHORIZATION.as_str())? {
        Some(authorization) => parse_authorization(authorization),
        None => {
            Err(report!(ReconAppError::Unauthorized).attach_printable("No credential provided"))
        }
    }
}

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
        let Credential::AdminApiKey(request_admin_api_key) = extract_credential(req.headers())?
        else {
            return Err(
                report!(ReconAppError::Unauthorized).attach_printable("Expected an admin key")
            );
        };
        let admin_api_key = &state.config.auth.get_inner().admin_api_key;

        if request_admin_api_key.peek() != admin_api_key.peek() {
            Err(report!(ReconAppError::Unauthorized).attach_printable("Admin Auth Failure"))?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct InternalApiKeyAuth;

impl AuthValidator for InternalApiKeyAuth {
    type AuthData = ();

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        let request_internal_api_key =
            get_internal_api_key(req.headers()).change_context(ReconAppError::Unauthorized)?;
        let internal_api_key = &state.config.auth.get_inner().internal_api_key;

        if request_internal_api_key != internal_api_key.peek() {
            Err(report!(ReconAppError::Unauthorized)
                .attach_printable("Internal API key auth failure"))?;
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

    /// Create JWT auth that validates profile_id matches the provided value and checks the permission
    pub fn with_profile(profile_id: String, permission: Permission) -> Self {
        Self {
            merchant_id: None,
            profile_id: Some(profile_id),
            permission: Some(permission),
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

        validate_merchant_and_profile(&state, &merchant_id, &profile_id).await?;

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
            auth_method: AuthMethod::Jwt,
        })
    }
}

async fn validate_merchant_and_profile(
    state: &AppState,
    merchant_id: &MerchantId,
    profile_id: &str,
) -> Result<(), ReconAppError> {
    state
        .store
        .find_merchant_account_by_id(merchant_id)
        .await
        .to_not_found_response(|| ReconAppError::Unauthorized)?;

    let business_profile = state
        .store
        .find_business_profile_by_id(profile_id)
        .await
        .to_not_found_response(|| ReconAppError::Unauthorized)?;

    if business_profile.merchant_id != *merchant_id {
        return Err(report!(ReconAppError::Unauthorized));
    }

    Ok(())
}

/// Every failure is the same `Unauthorized`. Logged reasons may carry the key `id`, never a header
/// value.
pub struct ApiKeyAuth;

impl ApiKeyAuth {
    async fn authenticate(
        &self,
        key: &Secret<String>,
        req: &HttpRequest,
        state: &AppState,
    ) -> Result<AuthContext, ReconAppError> {
        let hashed_api_key =
            PlaintextApiKey::from_request(key.peek()).hash(&state.api_key_hash_key);

        let api_key = state
            .store
            .find_active_api_key_by_hash(hashed_api_key.peek())
            .await
            .to_not_found_response(|| ReconAppError::Unauthorized)?;
        let key_id = &api_key.id;

        if api_key
            .expires_at
            .is_some_and(|expires_at| expires_at <= date_time::now())
        {
            return Err(report!(ReconAppError::Unauthorized)
                .attach_printable(format!("API key {key_id} has expired")));
        }

        let profile_id = get_header_value_by_key(auth_headers::X_PROFILE_ID.into(), req.headers())?
            .get_required_value(auth_headers::X_PROFILE_ID)
            .change_context(ReconAppError::Unauthorized)?
            .to_string();

        validate_merchant_and_profile(state, &api_key.merchant_id, &profile_id)
            .await
            .attach_printable_lazy(|| format!("API key {key_id} used with an invalid profile"))?;

        Ok(AuthContext {
            user_id: format!("apikey:{key_id}"),
            merchant_id: api_key.merchant_id,
            profile_id,
            auth_method: AuthMethod::ApiKey,
        })
    }
}

impl AuthValidator for ApiKeyAuth {
    type AuthData = AuthContext;

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        match extract_credential(req.headers())? {
            Credential::ApiKey(key) => self.authenticate(&key, req, &state).await,
            _ => Err(report!(ReconAppError::Unauthorized).attach_printable("Expected an API key")),
        }
    }
}

pub struct JwtOrApiKey {
    jwt: JwtAuth,
    api_key: ApiKeyAuth,
}

impl JwtOrApiKey {
    pub fn new(permission: Permission) -> Self {
        Self {
            jwt: JwtAuth::basic(permission),
            api_key: ApiKeyAuth,
        }
    }
}

impl AuthValidator for JwtOrApiKey {
    type AuthData = AuthContext;

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        match extract_credential(req.headers())? {
            Credential::Bearer => self.jwt.validate(req, state).await,
            Credential::ApiKey(key) => self.api_key.authenticate(&key, req, &state).await,
            Credential::AdminApiKey(_) => Err(report!(ReconAppError::Unauthorized)
                .attach_printable("Admin key is not accepted on this route")),
        }
    }
}

#[derive(Debug)]
pub struct ManagementAuthContext {
    pub merchant_id: MerchantId,
    pub user_id: Option<String>,
}

/// Admin API key plus mandatory `x-merchant-id`. No permission is checked.
#[derive(Debug, Default)]
pub struct AdminApiAuthWithMerchantIdFromHeader;

impl AuthValidator for AdminApiAuthWithMerchantIdFromHeader {
    type AuthData = MerchantId;

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        AdminApiAuth.validate(req, state.clone()).await?;

        let merchant_id =
            get_header_value_by_key(auth_headers::X_MERCHANT_ID.into(), req.headers())?
                .get_required_value(auth_headers::X_MERCHANT_ID)
                .change_context(ReconAppError::Unauthorized)?;
        let merchant_id = MerchantId::wrap(merchant_id.to_string())
            .change_context(ReconAppError::Unauthorized)?;

        state
            .store
            .find_merchant_account_by_id(&merchant_id)
            .await
            .to_not_found_response(|| ReconAppError::Unauthorized)?;

        Ok(merchant_id)
    }
}

pub struct JwtOrAdminApiAuth {
    jwt: JwtAuth,
}

impl JwtOrAdminApiAuth {
    pub fn new(permission: Permission) -> Self {
        Self {
            jwt: JwtAuth::basic(permission),
        }
    }
}

impl AuthValidator for JwtOrAdminApiAuth {
    type AuthData = ManagementAuthContext;

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        match extract_credential(req.headers())? {
            Credential::Bearer => {
                let auth = self.jwt.validate(req, state).await?;
                Ok(ManagementAuthContext {
                    merchant_id: auth.merchant_id,
                    user_id: Some(auth.user_id),
                })
            }
            Credential::AdminApiKey(_) => {
                let merchant_id = AdminApiAuthWithMerchantIdFromHeader
                    .validate(req, state)
                    .await?;
                Ok(ManagementAuthContext {
                    merchant_id,
                    user_id: None,
                })
            }
            Credential::ApiKey(_) => Err(report!(ReconAppError::Unauthorized)
                .attach_printable("An API key can't manage API keys")),
        }
    }
}

pub fn get_internal_api_key(headers: &HeaderMap) -> Result<&str, ReconAppError> {
    get_header_value_by_key(auth_headers::X_INTERNAL_API_KEY.into(), headers)?
        .get_required_value("x-internal-api-key")
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
