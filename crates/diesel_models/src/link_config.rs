use crate::schema::link_config;
use common_types::link_config::{
    link_matching_key::LinkSearchIdentifier, link_trigger::LinkTrigger,
};
use diesel::{Insertable, Queryable};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Queryable)]
#[diesel(table_name = link_config)]
pub struct LinkConfig {
    pub id: String,
    pub profile_id: String,
    pub account_id: String,
    pub transaction_rule_id: String,
    pub confirmation_rule_id: String,
    pub transaction_trigger: LinkTrigger,
    pub confirmation_trigger: LinkTrigger,
    pub link_search_identifier: LinkSearchIdentifier,
    pub priority: i16,
    pub is_active: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = link_config)]
pub struct LinkConfigNew {
    pub id: String,
    pub profile_id: String,
    pub account_id: String,
    pub transaction_rule_id: String,
    pub confirmation_rule_id: String,
    pub transaction_trigger: LinkTrigger,
    pub confirmation_trigger: LinkTrigger,
    pub link_search_identifier: LinkSearchIdentifier,
    pub priority: i16,
    pub is_active: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}
