use actix_web::{http::header::HeaderMap, HttpRequest};
use common_types::permissions::Permission;
use common_utils::{
    auth::{Actor, AuthContext},
    errors::ReportSwitchExt,
    id_type::MerchantId,
};
use error_stack::ResultExt;
use external_services::user::{
    interfaces::UserInterface, types::request::AuthorizeUserTokenRequest,
};
use http::header;
use jsonwebtoken::{decode, DecodingKey, Validation};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use storage_impl::{
    business_profile::BusinessProfileInterface, merchant_account::MerchantAccountInterface,
};

use super::option_ext::OptionExt;

use crate::{
    errors::{conversions::StorageErrorExt, ReconAppError},
    routes::AppState,
};

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

pub mod auth_headers {
    pub const X_INTERNAL_API_KEY: &str = "x-internal-api-key";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub exp: u64,
}

enum Credential {
    Jwt(Secret<String>),
    AdminApiKey(Secret<String>),
}

impl Credential {
    fn from_headers(headers: &HeaderMap) -> Result<Self, ReconAppError> {
        let value = headers
            .get(header::AUTHORIZATION)
            .ok_or_else(|| {
                error_stack::Report::new(ReconAppError::Unauthorized)
                    .attach("Missing Authorization header")
            })?
            .to_str()
            .change_context(ReconAppError::Unauthorized)?;

        if let Some(token) = value.strip_prefix("Bearer ") {
            Ok(Self::Jwt(Secret::new(token.to_string())))
        } else if let Some(key) = value.strip_prefix("admin-api-key=") {
            Ok(Self::AdminApiKey(Secret::new(key.to_string())))
        } else {
            Err(error_stack::Report::new(ReconAppError::Unauthorized)
                .attach("Unknown credential scheme"))
        }
    }
}

pub trait AuthValidator {
    type AuthData: Send;

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError>;
}

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
        let request_admin_api_key = match Credential::from_headers(req.headers())? {
            Credential::AdminApiKey(key) => key,
            Credential::Jwt(_) => {
                return Err(error_stack::Report::new(ReconAppError::Unauthorized)
                    .attach("Expected an admin API key"));
            }
        };
        let admin_api_key = &state.config.auth.get_inner().admin_api_key;

        if request_admin_api_key.peek() != admin_api_key.peek() {
            Err(error_stack::Report::new(ReconAppError::Unauthorized).attach("Admin Auth Failure"))?;
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
            Err(error_stack::Report::new(ReconAppError::Unauthorized)
                .attach("Internal API key auth failure"))?;
        }

        Ok(())
    }
}

#[derive(Debug)]
enum JwtRule {
    Permission(Permission),
    WithoutPermission,
}

#[derive(Debug)]
pub struct UserAuth {
    jwt: JwtRule,
    required_merchant_id: Option<MerchantId>,
    required_profile_id: Option<String>,
}

impl UserAuth {
    pub fn jwt(permission: Permission) -> Self {
        Self::with_jwt_rule(JwtRule::Permission(permission))
    }

    pub fn jwt_without_permission() -> Self {
        Self::with_jwt_rule(JwtRule::WithoutPermission)
    }

    fn with_jwt_rule(jwt: JwtRule) -> Self {
        Self {
            jwt,
            required_merchant_id: None,
            required_profile_id: None,
        }
    }

    pub fn require_merchant_id(mut self, merchant_id: MerchantId) -> Self {
        self.required_merchant_id = Some(merchant_id);
        self
    }

    pub fn require_profile_id(mut self, profile_id: String) -> Self {
        self.required_profile_id = Some(profile_id);
        self
    }

    async fn authenticate_jwt(
        &self,
        token: &Secret<String>,
        state: &AppState,
    ) -> Result<AuthContext, ReconAppError> {
        let decoding_key =
            DecodingKey::from_secret(state.config.auth.get_inner().jwt_secret.peek().as_ref());
        let claims = decode::<Claims>(token.peek(), &decoding_key, &Validation::default())
            .map(|data| data.claims)
            .change_context(ReconAppError::InvalidToken)?;

        if claims.user_id.is_empty() {
            return Err(error_stack::Report::new(ReconAppError::Unauthorized)
                .attach("Missing user_id in JWT claims"));
        }

        if self
            .required_merchant_id
            .as_ref()
            .is_some_and(|merchant_id| *merchant_id != claims.merchant_id)
        {
            return Err(error_stack::Report::new(ReconAppError::Unauthorized)
                .attach("Merchant does not match the route"));
        }

        if self
            .required_profile_id
            .as_deref()
            .is_some_and(|profile_id| profile_id != claims.profile_id)
        {
            return Err(error_stack::Report::new(ReconAppError::Unauthorized)
                .attach("Profile does not match the route"));
        }

        state
            .store
            .find_merchant_account_by_id(&claims.merchant_id)
            .await
            .to_not_found_response(|| ReconAppError::Unauthorized)?;

        let business_profile = state
            .store
            .find_business_profile_by_id(&claims.profile_id)
            .await
            .to_not_found_response(|| ReconAppError::Unauthorized)?;

        if business_profile.merchant_id != claims.merchant_id {
            return Err(error_stack::Report::new(ReconAppError::Unauthorized)
                .attach("Profile does not belong to merchant"));
        }

        if let JwtRule::Permission(permission) = self.jwt {
            state
                .user_client
                .authorize_user_token(
                    claims.merchant_id.clone(),
                    claims.profile_id.clone(),
                    AuthorizeUserTokenRequest {
                        token: token.clone(),
                        permission,
                    },
                )
                .await
                .switch()?;
        }

        Ok(AuthContext {
            merchant_id: claims.merchant_id,
            profile_id: claims.profile_id,
            actor: Actor::User {
                user_id: claims.user_id,
            },
        })
    }
}

impl AuthValidator for UserAuth {
    type AuthData = AuthContext;

    async fn validate(
        &self,
        req: &HttpRequest,
        state: AppState,
    ) -> Result<Self::AuthData, ReconAppError> {
        match Credential::from_headers(req.headers())? {
            Credential::Jwt(token) => self.authenticate_jwt(&token, &state).await,
            Credential::AdminApiKey(_) => {
                Err(error_stack::Report::new(ReconAppError::Unauthorized)
                    .attach("Admin API key is not accepted on this route"))
            }
        }
    }
}

pub fn get_internal_api_key(headers: &HeaderMap) -> Result<&str, ReconAppError> {
    get_header_value_by_key(auth_headers::X_INTERNAL_API_KEY.into(), headers)?
        .get_required_value("x-internal-api-key")
}

fn get_header_value_by_key(
    key: String,
    headers: &HeaderMap,
) -> Result<Option<&str>, ReconAppError> {
    headers
        .get(&key)
        .map(|source_str| {
            source_str
                .to_str()
                .change_context(ReconAppError::InternalServerError)
                .attach(format!(
                    "Failed to convert header value to string for header key: {key}"
                ))
        })
        .transpose()
}
