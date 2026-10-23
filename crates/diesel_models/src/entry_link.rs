use common_enums::EntryLinkStatus;
use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel::{Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::schema::entry_link;

#[derive(Clone, Debug, Queryable)]
#[diesel(table_name = entry_link)]
pub struct EntryLink {
    pub id: String,
    pub profile_id: String,
    pub account_id: String,
    pub link_config_id: String,
    pub transaction_entry_id: String,
    pub confirmation_entry_id: String,
    pub status: EntryLinkStatus,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = entry_link)]
pub struct EntryLinkNew {
    pub id: String,
    pub profile_id: String,
    pub account_id: String,
    pub link_config_id: String,
    pub transaction_entry_id: String,
    pub confirmation_entry_id: String,
    pub status: EntryLinkStatus,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

impl EntryLinkNew {
    pub fn new(
        profile_id: String,
        account_id: String,
        link_config_id: String,
        transaction_entry_id: String,
        confirmation_entry_id: String,
        status: EntryLinkStatus,
    ) -> Self {
        let now = date_time::now();

        Self {
            id: generate_id_with_default_len("elink"),
            profile_id,
            account_id,
            link_config_id,
            transaction_entry_id,
            confirmation_entry_id,
            status,
            created_at: now,
            last_modified_at: now,
        }
    }
}
