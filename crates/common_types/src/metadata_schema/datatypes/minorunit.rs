use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::units::MinorUnit;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "minor_unit_config_version", rename_all = "snake_case")]
pub enum MinorUnitConfig {
    V1(MinorUnitConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MinorUnitConfigV1 {
    pub validation_rules: Vec<MinorUnitValidationRule>,
    #[serde(default)]
    pub transformation_rules: Vec<MinorUnitTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "minor_unit_validation_rule_version", rename_all = "snake_case")]
pub enum MinorUnitValidationRule {
    V1(MinorUnitValidationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "minor_unit_validation_rule_type", rename_all = "snake_case")]
pub enum MinorUnitValidationRuleV1 {
    MinValue { value: MinorUnit },
    MaxValue { value: MinorUnit },
    PositiveOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(
    tag = "minor_unit_transformation_rule_version",
    rename_all = "snake_case"
)]
pub enum MinorUnitTransformationRule {
    V1(MinorUnitTransformationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "minor_unit_transformation_rule_type", rename_all = "snake_case")]
pub enum MinorUnitTransformationRuleV1 {
    Trim,
    JsonExtract { pointer: String },
    Absolute,
}
