use common_enums::ApiKeyStatus;
use common_utils::id_type::MerchantId;
use masking::StrongSecret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateApiKeyRequest {
    #[schema(example = "Reporting integration")]
    pub name: String,
    #[schema(example = "Used by the nightly reporting job")]
    pub description: Option<String>,
    #[schema(example = "2027-01-01T00:00:00.000Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateApiKeyRequest {
    #[serde(flatten)]
    pub update_type: UpdateApiKeyType,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "update_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum UpdateApiKeyType {
    RevokeUpdate {},
    DetailsUpdate {
        #[schema(example = "Reporting integration")]
        name: Option<String>,
        #[schema(example = "Used by the nightly reporting job")]
        description: Option<String>,
    },
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
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<PrimitiveDateTime>,
    #[schema(example = "2026-01-01T00:00:00.000Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateApiKeyResponse {
    #[serde(flatten)]
    pub api_key_details: ApiKeyResponse,
    #[schema(value_type = String, example = "dev_aB3dE9...")]
    pub api_key: StrongSecret<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListApiKeysResponse {
    pub api_keys: Vec<ApiKeyResponse>,
}
