pub mod aging_config;
pub mod entry_field;
pub mod grouping_field;
pub mod match_rules;
pub mod search_identifier;
pub mod strategies;
pub mod tolerance_config;
pub mod trigger;

use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::strategies::{
    one_to_many_strategy::OneToManyStrategy, one_to_one_strategy::OneToOneStrategy,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "recon_strategy_version", rename_all = "snake_case")]
pub enum ReconStrategy {
    V1(ReconStrategyV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "recon_strategy_type", rename_all = "snake_case")]
pub enum ReconStrategyV1 {
    OneToOne(OneToOneStrategy),
    OneToMany(OneToManyStrategy),
}

common_utils::impl_to_sql_from_sql_json!(ReconStrategy);
