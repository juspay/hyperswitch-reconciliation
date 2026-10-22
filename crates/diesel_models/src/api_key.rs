use common_enums::ApiKeyStatus;
use common_utils::{date_time, id_type::MerchantId};
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
    pub created_by: String,
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
    pub created_by: String,
    pub created_at: PrimitiveDateTime,
    pub expires_at: Option<PrimitiveDateTime>,
    pub last_modified_at: PrimitiveDateTime,
    pub status: ApiKeyStatus,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = api_key)]
pub struct ApiKeyUpdateInternal {
    pub name: Option<String>,
    // `Some(None)` clears the column, `None` leaves it untouched.
    pub description: Option<Option<String>>,
    pub expires_at: Option<Option<PrimitiveDateTime>>,
    pub last_modified_at: PrimitiveDateTime,
    pub status: Option<ApiKeyStatus>,
}

#[derive(Debug, Clone)]
pub enum ApiKeyUpdate {
    Update {
        name: Option<String>,
        description: Option<Option<String>>,
        expires_at: Option<Option<PrimitiveDateTime>>,
    },
    /// Revoke the key (soft delete).
    Revoke,
}

impl From<ApiKeyUpdate> for ApiKeyUpdateInternal {
    fn from(api_key_update: ApiKeyUpdate) -> Self {
        match api_key_update {
            ApiKeyUpdate::Update {
                name,
                description,
                expires_at,
            } => Self {
                name,
                description,
                expires_at,
                last_modified_at: date_time::now(),
                status: None,
            },
            ApiKeyUpdate::Revoke => Self {
                name: None,
                description: None,
                expires_at: None,
                last_modified_at: date_time::now(),
                status: Some(ApiKeyStatus::Revoked),
            },
        }
    }
}
