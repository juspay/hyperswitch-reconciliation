use crate::schema::link_config;
use common_types::link_config::{
    link_matching_key::LinkSearchIdentifier, link_trigger::LinkTrigger,
};
use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel::{AsChangeset, Insertable, Queryable};
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

impl LinkConfigNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        profile_id: String,
        account_id: String,
        transaction_rule_id: String,
        confirmation_rule_id: String,
        transaction_trigger: LinkTrigger,
        confirmation_trigger: LinkTrigger,
        link_search_identifier: LinkSearchIdentifier,
        priority: u8,
    ) -> Self {
        let now = date_time::now();
        Self {
            id: generate_id_with_default_len("link_config"),
            profile_id,
            account_id,
            transaction_rule_id,
            confirmation_rule_id,
            transaction_trigger,
            confirmation_trigger,
            link_search_identifier,
            priority: i16::from(priority),
            is_active: true,
            created_at: now,
            last_modified_at: now,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = link_config)]
pub struct LinkConfigUpdateInternal {
    pub transaction_rule_id: Option<String>,
    pub confirmation_rule_id: Option<String>,
    pub transaction_trigger: Option<LinkTrigger>,
    pub confirmation_trigger: Option<LinkTrigger>,
    pub link_search_identifier: Option<LinkSearchIdentifier>,
    pub priority: Option<i16>,
    pub is_active: Option<bool>,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Clone)]
pub enum LinkConfigUpdate {
    RuleIdsUpdate {
        transaction_rule_id: Option<String>,
        confirmation_rule_id: Option<String>,
    },
    TriggerUpdate {
        transaction_trigger: Option<LinkTrigger>,
        confirmation_trigger: Option<LinkTrigger>,
    },
    SearchIdentifierUpdate {
        link_search_identifier: LinkSearchIdentifier,
    },
    PriorityUpdate {
        priority: i16,
    },
    ActiveUpdate {
        is_active: bool,
    },
}

impl From<LinkConfigUpdate> for LinkConfigUpdateInternal {
    fn from(update: LinkConfigUpdate) -> Self {
        let last_modified_at = date_time::now();

        match update {
            LinkConfigUpdate::RuleIdsUpdate {
                transaction_rule_id,
                confirmation_rule_id,
            } => Self {
                transaction_rule_id,
                confirmation_rule_id,
                transaction_trigger: None,
                confirmation_trigger: None,
                link_search_identifier: None,
                priority: None,
                is_active: None,
                last_modified_at,
            },
            LinkConfigUpdate::TriggerUpdate {
                transaction_trigger,
                confirmation_trigger,
            } => Self {
                transaction_rule_id: None,
                confirmation_rule_id: None,
                transaction_trigger,
                confirmation_trigger,
                link_search_identifier: None,
                priority: None,
                is_active: None,
                last_modified_at,
            },
            LinkConfigUpdate::SearchIdentifierUpdate {
                link_search_identifier,
            } => Self {
                transaction_rule_id: None,
                confirmation_rule_id: None,
                transaction_trigger: None,
                confirmation_trigger: None,
                link_search_identifier: Some(link_search_identifier),
                priority: None,
                is_active: None,
                last_modified_at,
            },
            LinkConfigUpdate::PriorityUpdate { priority } => Self {
                transaction_rule_id: None,
                confirmation_rule_id: None,
                transaction_trigger: None,
                confirmation_trigger: None,
                link_search_identifier: None,
                priority: Some(priority),
                is_active: None,
                last_modified_at,
            },
            LinkConfigUpdate::ActiveUpdate { is_active } => Self {
                transaction_rule_id: None,
                confirmation_rule_id: None,
                transaction_trigger: None,
                confirmation_trigger: None,
                link_search_identifier: None,
                priority: None,
                is_active: Some(is_active),
                last_modified_at,
            },
        }
    }
}

#[derive(Default)]
pub struct LinkConfigListFilter {
    pub account_id: Option<String>,
}
