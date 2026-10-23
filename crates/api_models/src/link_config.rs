use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

pub mod link_matching_key;
pub mod link_trigger;

pub use link_matching_key::LinkSearchIdentifier;
pub use link_trigger::LinkTrigger;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkConfigCreateRequest {
    pub account_id: String,
    pub transaction_rule_id: String,
    pub confirmation_rule_id: String,
    pub transaction_trigger: LinkTrigger,
    pub confirmation_trigger: LinkTrigger,
    pub link_search_identifier: LinkSearchIdentifier,
    pub priority: u8,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct LinkConfigUpdateRequest {
    pub transaction_rule_id: Option<String>,
    pub confirmation_rule_id: Option<String>,
    pub transaction_trigger: Option<LinkTrigger>,
    pub confirmation_trigger: Option<LinkTrigger>,
    pub link_search_identifier: Option<LinkSearchIdentifier>,
    pub priority: Option<u8>,
    pub is_active: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GetLinkConfigResponse {
    pub link_config_id: String,
    pub profile_id: String,
    pub account_id: String,
    pub transaction_rule_id: String,
    pub confirmation_rule_id: String,
    pub transaction_trigger: LinkTrigger,
    pub confirmation_trigger: LinkTrigger,
    pub link_search_identifier: LinkSearchIdentifier,
    pub priority: i16,
    pub is_active: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
}
