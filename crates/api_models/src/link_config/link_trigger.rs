use common_types::recon_rule::entry_field::EntryField;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkTrigger {
    pub any: Vec<LinkTriggerGroup>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkTriggerGroup {
    pub all: Vec<LinkTriggerCondition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkTriggerCondition {
    pub field: EntryField,
    pub operator: LinkTriggerOperator,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkTriggerOperator {
    Equals,
    NotEquals,
}
