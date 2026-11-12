use common_enums::ApiKeyStatus;
use common_utils::{date_time, id_generation::generate_id_with_default_len, id_type::MerchantId};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;
use time::PrimitiveDateTime;

use crate::schema::api_key;

/// A merchant-level API key. Only a keyed hash of the key is stored, never the plaintext.
///
/// Deliberately not `Serialize`/`Deserialize`, so the hash can't end up in an API response by accident.
#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = api_key, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct ApiKey {
    pub id: String,
    pub merchant_id: MerchantId,
    pub name: String,
    pub description: Option<String>,
    pub hashed_api_key: Secret<String>,
    pub prefix: String,
    pub created_by: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub expires_at: Option<PrimitiveDateTime>,
    pub last_modified_at: PrimitiveDateTime,
    pub status: ApiKeyStatus,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = api_key, check_for_backend(diesel::pg::Pg))]
pub struct ApiKeyNew {
    pub id: String,
    pub merchant_id: MerchantId,
    pub name: String,
    pub description: Option<String>,
    pub hashed_api_key: Secret<String>,
    pub prefix: String,
    pub created_by: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub expires_at: Option<PrimitiveDateTime>,
    pub last_modified_at: PrimitiveDateTime,
    pub status: ApiKeyStatus,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = api_key)]
pub struct ApiKeyUpdateInternal {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<ApiKeyStatus>,
    pub last_modified_at: PrimitiveDateTime,
}

impl ApiKeyNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        merchant_id: MerchantId,
        name: String,
        description: Option<String>,
        hashed_api_key: Secret<String>,
        prefix: String,
        created_by: Option<String>,
        expires_at: Option<PrimitiveDateTime>,
        now: PrimitiveDateTime,
    ) -> Self {
        Self {
            id: generate_id_with_default_len("apikey"),
            merchant_id,
            name,
            description,
            hashed_api_key,
            prefix,
            created_by,
            created_at: now,
            expires_at,
            last_modified_at: now,
            status: ApiKeyStatus::Active,
        }
    }
}

/// `expires_at` is deliberately not updatable.
#[derive(Debug, Clone)]
pub enum ApiKeyUpdate {
    DetailsUpdate {
        name: Option<String>,
        description: Option<String>,
    },
    RevokeUpdate,
}

impl From<ApiKeyUpdate> for ApiKeyUpdateInternal {
    fn from(api_key_update: ApiKeyUpdate) -> Self {
        let last_modified_at = date_time::now();

        match api_key_update {
            ApiKeyUpdate::DetailsUpdate { name, description } => Self {
                name,
                description,
                status: None,
                last_modified_at,
            },
            ApiKeyUpdate::RevokeUpdate => Self {
                name: None,
                description: None,
                status: Some(ApiKeyStatus::Revoked),
                last_modified_at,
            },
        }
    }
}
