use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::EntryField;

/// Versioned match rules to support future rule types
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "match_version", rename_all = "snake_case")]
pub enum MatchRules {
    V1(MatchRulesV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MatchRulesV1 {
    pub rules: Vec<MatchRuleV1>,
}

/// Match rule for comparing source and target fields using type-safe EntryField
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MatchRuleV1 {
    pub source_field: EntryField,
    pub target_field: EntryField,
    pub operator: MatchOperator,
    #[serde(default)]
    pub label: Option<String>,
}

/// Match operator for field comparison (currently only equals)
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchOperator {
    Equals,
}

common_utils::impl_to_sql_from_sql_json!(MatchRules);
