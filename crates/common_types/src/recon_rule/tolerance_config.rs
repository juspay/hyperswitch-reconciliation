use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::units::MinorUnit;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[serde(tag = "tolerance_config_version", rename_all = "snake_case")]
#[diesel(sql_type = Jsonb)]
pub enum ToleranceConfig {
    V1(ToleranceConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ToleranceConfigV1 {
    pub lower_tolerance: MinorUnit,
    pub upper_tolerance: MinorUnit,
    pub tolerance_account_id: String,
}

common_utils::impl_to_sql_from_sql_json!(ToleranceConfig);
