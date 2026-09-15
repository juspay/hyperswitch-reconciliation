use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "number_config_version", rename_all = "snake_case")]
pub enum NumberConfig {
    V1(NumberConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct NumberConfigV1 {
    pub validation_rules: Vec<NumberValidationRule>,
    #[serde(default)]
    pub transformation_rules: Vec<NumberTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "number_validation_rule_version", rename_all = "snake_case")]
pub enum NumberValidationRule {
    V1(NumberValidationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "number_validation_rule_type", rename_all = "snake_case")]
pub enum NumberValidationRuleV1 {
    MinValue { value: f64 },
    MaxValue { value: f64 },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "number_transformation_rule_version", rename_all = "snake_case")]
pub enum NumberTransformationRule {
    V1(NumberTransformationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "number_transformation_rule_type", rename_all = "snake_case")]
pub enum NumberTransformationRuleV1 {
    Trim,
    JsonExtract { pointer: String },
}
