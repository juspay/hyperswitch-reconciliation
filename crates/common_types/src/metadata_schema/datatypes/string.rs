use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::metadata_schema::datatypes::majorunit::ReplaceMode;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "string_config_version", rename_all = "snake_case")]
pub enum StringConfig {
    V1(StringConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StringConfigV1 {
    pub validation_rules: Vec<StringValidationRule>,
    #[serde(default)]
    pub transformation_rules: Vec<StringTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "string_validation_rule_version", rename_all = "snake_case")]
pub enum StringValidationRule {
    V1(StringValidationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "string_validation_rule_type", rename_all = "snake_case")]
pub enum StringValidationRuleV1 {
    MaxLength { value: usize },
    MinLength { value: usize },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "string_transformation_rule_version", rename_all = "snake_case")]
pub enum StringTransformationRule {
    V1(StringTransformationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "string_transformation_rule_type", rename_all = "snake_case")]
pub enum StringTransformationRuleV1 {
    DefaultValue {
        value: String,
    },
    ToUpperCase,
    ToLowerCase,
    StripPrefix {
        prefix: String,
    },
    StripSuffix {
        suffix: String,
    },
    Trim,
    JsonExtract {
        pointer: String,
    },
    Regex {
        pattern: String,
        group: Option<usize>,
    },
    ReplaceChar {
        from: char,
        to: Option<char>,
        mode: ReplaceMode,
    },
}
