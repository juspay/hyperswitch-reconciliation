use common_utils::{
    errors::{CustomResult, ErrorSwitchFrom},
    id_type::MerchantId,
};
use error_stack::{Report, ResultExt};
use hyper::{
    body::Bytes,
    header::{HeaderName, HeaderValue},
};
use masking::{PeekInterface, Secret};

use crate::user::types::error::UserClientError;

pub mod hyperswitch_headers {
    pub const X_INTERNAL_API_KEY: &str = "x-internal-api-key";
    pub const X_MERCHANT_ID: &str = "x-merchant-id";
    pub const X_PROFILE_ID: &str = "x-profile-id";
    pub const CONTENT_TYPE: &str = "content-type";
    pub const X_TENANT_ID: &str = "x-tenant-id";
}

pub const HYPERSWITCH_TENANT_ID: &str = "public";

pub(crate) enum HyperswitchPath {
    UserById { user_id: String },
    ListUsers,
    ListAssignees { access_type: EntityType },
    AuthorizeUserToken,
}

impl HyperswitchPath {
    pub fn to_path(&self, base_url: String) -> String {
        match self {
            Self::UserById { user_id } => {
                format!("{}/user/internal/user/{}", base_url, user_id)
            }
            Self::ListUsers => format!("{}/user/internal/user/list", base_url),
            Self::ListAssignees { access_type } => {
                format!(
                    "{}/user/internal/members?access_level={}",
                    base_url, access_type
                )
            }
            Self::AuthorizeUserToken => format!("{}/user/internal/authorize", base_url),
        }
    }
}

pub(crate) enum HyperswitchAuth {
    InternalApiKey {
        internal_api_key: Secret<String>,
        merchant_id: MerchantId,
        profile_id: String,
    },
}

impl HyperswitchAuth {
    pub fn get_auth_headers(&self) -> HyperswitchResult<Vec<(HeaderName, HeaderValue)>> {
        match self {
            Self::InternalApiKey {
                internal_api_key,
                merchant_id,
                profile_id,
            } => Ok(vec![
                build_header(
                    HeaderName::from_static(hyperswitch_headers::X_INTERNAL_API_KEY),
                    internal_api_key.peek(),
                )?,
                build_header(
                    HeaderName::from_static(hyperswitch_headers::X_MERCHANT_ID),
                    merchant_id.get_string_repr(),
                )?,
                build_header(
                    HeaderName::from_static(hyperswitch_headers::X_PROFILE_ID),
                    profile_id,
                )?,
            ]),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum StandardHeader {
    TenantIdPublic,
}

impl StandardHeader {
    pub(crate) fn pair(self) -> (HeaderName, HeaderValue) {
        match self {
            Self::TenantIdPublic => (
                HeaderName::from_static(hyperswitch_headers::X_TENANT_ID),
                HeaderValue::from_static(HYPERSWITCH_TENANT_ID),
            ),
        }
    }
}

#[derive(Clone, Copy, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum EntityType {
    Profile,
    Merchant,
    Organization,
    Tenant,
}

#[derive(Debug, thiserror::Error)]
pub enum HyperswitchClientError {
    #[error("Failed to build the reqwest client ")]
    ClientConstructionFailed,
    #[error("Internal Server Error Received {0:?}")]
    InternalServerError(Bytes),
    #[error("URL encoding of request failed")]
    UrlEncodingFailed,
    #[error("Failed to construct header from the given value")]
    FailedtoConstructHeader,
    #[error("Failed to send request to Hyperswitch")]
    RequestNotSent(String),
    #[error("Response Decoding failed")]
    ResponseDecodingFailed,
    #[error("Bad request received {0:?}")]
    BadRequest(Bytes),
    #[error("Unexpected Error occurred while calling the Hyperswitch")]
    Unexpected(Bytes),
    #[error("Invalid JWT token was used")]
    Unauthorized(Bytes),
    #[error("Not authorized to access this resource")]
    Forbidden(Bytes),
}

impl ErrorSwitchFrom<HyperswitchClientError> for UserClientError {
    fn switch_from(value: &HyperswitchClientError) -> Self {
        match &value {
            HyperswitchClientError::Unauthorized(_) => Self::Unauthorized,
            HyperswitchClientError::Forbidden(_) => Self::Forbidden,
            HyperswitchClientError::InternalServerError(_)
            | HyperswitchClientError::Unexpected(_)
            | HyperswitchClientError::BadRequest(_)
            | HyperswitchClientError::ResponseDecodingFailed
            | HyperswitchClientError::ClientConstructionFailed
            | HyperswitchClientError::FailedtoConstructHeader
            | HyperswitchClientError::UrlEncodingFailed
            | HyperswitchClientError::RequestNotSent(_) => Self::InternalServerError,
        }
    }
}

pub type HyperswitchResult<T> = Result<T, Report<HyperswitchClientError>>;

pub fn build_header(
    name: HeaderName,
    value: &str,
) -> CustomResult<(HeaderName, HeaderValue), HyperswitchClientError> {
    let value = HeaderValue::from_str(value)
        .change_context(HyperswitchClientError::FailedtoConstructHeader)
        .attach_printable_lazy(|| format!("Failed to construct header value for `{}`", name))?;
    Ok((name, value))
}
