use common_enums::{EntryLinkKeyStatus, EntryLinkSide};
use common_types::field_path_key::FieldPathKey;
use common_utils::{
    date_time, encryption::Encryption, id_generation::generate_id_with_default_len,
};
use diesel::{AsChangeset, Insertable, Queryable};
use masking::Secret;
use time::PrimitiveDateTime;

use crate::schema::entry_link_key;

#[derive(Clone, Debug, Queryable)]
#[diesel(table_name = entry_link_key)]
pub struct EntryLinkKey {
    pub id: String,
    pub profile_id: String,
    pub account_id: String,
    pub link_config_id: String,
    pub entry_id: String,
    pub rule_id: String,
    pub target_side: EntryLinkSide,
    pub status: EntryLinkKeyStatus,
    pub target_key: FieldPathKey,
    pub encrypted_value: Encryption,
    pub hashed_value: Secret<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = entry_link_key)]
pub struct EntryLinkKeyNew {
    pub id: String,
    pub profile_id: String,
    pub account_id: String,
    pub link_config_id: String,
    pub entry_id: String,
    pub rule_id: String,
    pub target_side: EntryLinkSide,
    pub status: EntryLinkKeyStatus,
    pub target_key: FieldPathKey,
    pub encrypted_value: Encryption,
    pub hashed_value: Secret<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

impl EntryLinkKeyNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        profile_id: String,
        account_id: String,
        link_config_id: String,
        entry_id: String,
        rule_id: String,
        target_side: EntryLinkSide,
        status: EntryLinkKeyStatus,
        target_key: FieldPathKey,
        encrypted_value: Encryption,
        hashed_value: Secret<String>,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("elkey");

        Self {
            id,
            profile_id,
            account_id,
            link_config_id,
            entry_id,
            rule_id,
            target_side,
            status,
            target_key,
            encrypted_value,
            hashed_value,
            created_at: now,
            last_modified_at: now,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = entry_link_key)]
pub struct EntryLinkKeyUpdateInternal {
    pub status: EntryLinkKeyStatus,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Clone)]
pub enum EntryLinkKeyUpdate {
    StatusUpdate { status: EntryLinkKeyStatus },
}

impl From<EntryLinkKeyUpdate> for EntryLinkKeyUpdateInternal {
    fn from(entry_link_key_update: EntryLinkKeyUpdate) -> Self {
        let last_modified_at = date_time::now();

        match entry_link_key_update {
            EntryLinkKeyUpdate::StatusUpdate { status } => Self {
                status,
                last_modified_at,
            },
        }
    }
}
