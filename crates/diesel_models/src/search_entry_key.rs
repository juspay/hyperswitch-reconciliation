use crate::schema::search_entry_key;
use diesel::prelude::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = search_entry_key)]
pub struct SearchEntryKey {
    pub id: String,
    pub profile_id: String,
    pub entry_id: String,
    pub primary_transaction_id: String,
    pub metadata_key: String,
    pub metadata_value: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = search_entry_key)]
pub struct SearchEntryKeyNew {
    pub id: String,
    pub profile_id: String,
    pub entry_id: String,
    pub primary_transaction_id: String,
    pub metadata_key: String,
    pub metadata_value: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}
