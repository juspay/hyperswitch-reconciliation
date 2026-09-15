use common_enums::SearchDuplicateStatus;
use common_types::field_path_key::FieldPathKey;
use common_utils::{
    date_time, encryption::Encryption, id_generation::generate_id_with_default_len,
    id_type::MerchantId,
};
use diesel::{prelude::AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::search_duplicate;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = search_duplicate)]
#[diesel(primary_key(id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchDuplicate {
    pub id: String,
    pub logical_staging_entry_id: String,
    pub account_id: String,
    pub transformation_config_id: String,
    pub field_path: FieldPathKey,
    pub encrypted_field_value: Encryption,
    pub hashed_field_value: Secret<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub merchant_id: MerchantId,
    pub primary_staging_entry_id: String,
    pub staging_entry_version: i32,
    pub status: SearchDuplicateStatus,
}

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = search_duplicate)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchDuplicateNew {
    pub id: String,
    pub logical_staging_entry_id: String,
    pub account_id: String,
    pub transformation_config_id: String,
    pub field_path: FieldPathKey,
    pub encrypted_field_value: Encryption,
    pub hashed_field_value: Secret<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub merchant_id: MerchantId,
    pub primary_staging_entry_id: String,
    pub staging_entry_version: i32,
    pub status: SearchDuplicateStatus,
}

impl SearchDuplicateNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        logical_staging_entry_id: String,
        account_id: String,
        merchant_id: MerchantId,
        transformation_config_id: String,
        field_path: FieldPathKey,
        encrypted_field_value: Encryption,
        hashed_field_value: Secret<String>,
        primary_staging_entry_id: String,
        staging_entry_version: i32,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("sea_dup");

        Self {
            id,
            logical_staging_entry_id,
            account_id,
            transformation_config_id,
            field_path,
            encrypted_field_value,
            hashed_field_value,
            created_at: now,
            last_modified_at: now,
            merchant_id,
            primary_staging_entry_id,
            staging_entry_version,
            status: SearchDuplicateStatus::Active,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = search_duplicate)]
pub struct SearchDuplicateUpdateInternal {
    pub status: Option<SearchDuplicateStatus>,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Clone)]
pub enum SearchDuplicateUpdate {
    StatusUpdate { status: SearchDuplicateStatus },
}

impl From<SearchDuplicateUpdate> for SearchDuplicateUpdateInternal {
    fn from(search_duplicate_update: SearchDuplicateUpdate) -> Self {
        let now = date_time::now();
        match search_duplicate_update {
            SearchDuplicateUpdate::StatusUpdate { status } => Self {
                status: Some(status),
                last_modified_at: now,
            },
        }
    }
}
