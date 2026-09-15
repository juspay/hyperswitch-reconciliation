use common_enums::SearchGroupStatus;
use common_utils::{
    date_time, encryption::Encryption, id_generation::generate_id_with_default_len,
    id_type::MerchantId,
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::search_group;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = search_group)]
#[diesel(primary_key(id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchGroup {
    pub id: String,
    pub rule_id: String,
    pub primary_transaction_id: String,
    pub encrypted_grouping_value: Encryption,
    pub hashed_grouping_value: Secret<String>,
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub status: SearchGroupStatus,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = search_group)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchGroupNew {
    pub id: String,
    pub rule_id: String,
    pub primary_transaction_id: String,
    pub encrypted_grouping_value: Encryption,
    pub hashed_grouping_value: Secret<String>,
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub status: SearchGroupStatus,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = search_group)]
pub struct SearchGroupUpdateInternal {
    pub status: SearchGroupStatus,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Clone)]
pub enum SearchGroupUpdate {
    DiscardUpdate,
}

impl From<SearchGroupUpdate> for SearchGroupUpdateInternal {
    fn from(search_group_update: SearchGroupUpdate) -> Self {
        let last_modified_at = date_time::now();

        match search_group_update {
            SearchGroupUpdate::DiscardUpdate => Self {
                status: SearchGroupStatus::Archived,
                last_modified_at,
            },
        }
    }
}

impl SearchGroupNew {
    pub fn new(
        rule_id: String,
        primary_transaction_id: String,
        encrypted_grouping_value: Encryption,
        hashed_grouping_value: Secret<String>,
        merchant_id: MerchantId,
        profile_id: String,
        status: SearchGroupStatus,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("sea_grp");

        Self {
            id,
            rule_id,
            primary_transaction_id,
            encrypted_grouping_value,
            hashed_grouping_value,
            merchant_id,
            profile_id,
            status,
            created_at: now,
            last_modified_at: now,
        }
    }
}
