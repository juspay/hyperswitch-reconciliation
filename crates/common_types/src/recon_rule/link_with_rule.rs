use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::EntryField;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[serde(tag = "link_with_rule_version", rename_all = "snake_case")]
#[diesel(sql_type = Jsonb)]
pub enum LinkWithRule {
    V1(LinkWithRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkWithRuleV1 {
    pub target_data: Vec<LinkWithRuleTargetData>,
    pub current_rule_account_id: String,
    pub current_rule_field: EntryField,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkWithRuleTargetData {
    pub rule_id: String,
    pub account_id: String,
    pub key: EntryField,
}

common_utils::impl_to_sql_from_sql_json!(LinkWithRule);
