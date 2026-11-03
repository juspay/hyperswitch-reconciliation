use common_enums::ApiKeyStatus;
use common_utils::id_type::MerchantId;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ApiKeyExpiration {
    Never(NeverMarker),
    DateTime(#[serde(with = "common_utils::custom_serde::iso8601")] PrimitiveDateTime),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NeverMarker {
    Never,
}

impl ApiKeyExpiration {
    pub const NEVER: Self = Self::Never(NeverMarker::Never);
}

impl From<ApiKeyExpiration> for Option<PrimitiveDateTime> {
    fn from(expiration: ApiKeyExpiration) -> Self {
        match expiration {
            ApiKeyExpiration::Never(_) => None,
            ApiKeyExpiration::DateTime(date_time) => Some(date_time),
        }
    }
}

impl From<Option<PrimitiveDateTime>> for ApiKeyExpiration {
    fn from(date_time: Option<PrimitiveDateTime>) -> Self {
        date_time.map_or(Self::NEVER, Self::DateTime)
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateApiKeyRequest {
    #[schema(example = "Reporting integration")]
    pub name: String,
    #[schema(example = "Used by the nightly reporting job")]
    pub description: Option<String>,
    /// A future datetime, or `"never"`
    #[schema(example = "2027-01-01T00:00:00.000Z")]
    pub expires_at: ApiKeyExpiration,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateApiKeyRequest {
    #[schema(example = "Reporting integration")]
    pub name: Option<String>,
    #[schema(example = "Used by the nightly reporting job")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyResponse {
    #[schema(example = "apikey_1234567890")]
    pub id: String,
    #[schema(value_type = String, example = "merchant_1234567890")]
    pub merchant_id: MerchantId,
    pub name: String,
    pub description: Option<String>,
    #[schema(example = "dev_aB3dE9")]
    pub prefix: String,
    #[schema(example = "user_1234567890")]
    pub created_by: Option<String>,
    #[schema(example = "active")]
    pub status: ApiKeyStatus,
    #[schema(example = "2026-01-01T00:00:00.000Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[schema(example = "2027-01-01T00:00:00.000Z")]
    pub expires_at: ApiKeyExpiration,
    #[schema(example = "2026-01-01T00:00:00.000Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateApiKeyResponse {
    #[serde(flatten)]
    pub api_key_details: ApiKeyResponse,
    #[schema(value_type = String, example = "dev_aB3dE9...")]
    pub api_key: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListApiKeysResponse {
    pub api_keys: Vec<ApiKeyResponse>,
}
