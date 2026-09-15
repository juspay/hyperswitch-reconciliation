use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "enum_config_version", rename_all = "snake_case")]
pub enum EnumConfig {
    V1(EnumConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EnumConfigV1 {
    pub mappings: HashMap<String, String>,
    #[serde(default)]
    pub transformation_rules: Vec<EnumTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "enum_transformation_rule_version", rename_all = "snake_case")]
pub enum EnumTransformationRule {
    V1(EnumTransformationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "enum_transformation_rule_type", rename_all = "snake_case")]
pub enum EnumTransformationRuleV1 {
    Trim,
    JsonExtract { pointer: String },
}
