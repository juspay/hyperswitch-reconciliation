use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::EntryField;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "trigger_version", rename_all = "snake_case")]
pub enum LinkTrigger {
    V1(LinkTriggerV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkTriggerV1 {
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

common_utils::impl_to_sql_from_sql_json!(LinkTrigger);
