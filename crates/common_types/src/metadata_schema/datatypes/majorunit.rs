use core::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::units::FloatMajorUnit;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "major_unit_config_version", rename_all = "snake_case")]
pub enum MajorUnitConfig {
    V1(MajorUnitConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MajorUnitConfigV1 {
    pub delimiter: AmountDelimiter,
    pub validation_rules: Vec<MajorUnitValidationRule>,
    #[serde(default)]
    pub transformation_rules: Vec<MajorUnitTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(
    tag = "major_unit_transformation_rule_version",
    rename_all = "snake_case"
)]
pub enum MajorUnitTransformationRule {
    V1(MajorUnitTransformationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "major_unit_transformation_rule_type", rename_all = "snake_case")]
pub enum MajorUnitTransformationRuleV1 {
    Trim,
    JsonExtract {
        pointer: String,
    },
    Negate,
    Absolute,
    ReplaceChar {
        from: char,
        to: Option<char>,
        mode: ReplaceMode,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "mode_type", rename_all = "snake_case")]
pub enum ReplaceMode {
    All,
    Single { occurrence: usize, from_end: bool },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AmountDelimiter {
    Dot,
    Comma,
}

impl fmt::Display for AmountDelimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dot => write!(f, "."),
            Self::Comma => write!(f, ","),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "major_unit_validation_rule_version", rename_all = "snake_case")]
pub enum MajorUnitValidationRule {
    V1(MajorUnitValidationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "major_unit_validation_rule_type", rename_all = "snake_case")]
pub enum MajorUnitValidationRuleV1 {
    MinValue { value: FloatMajorUnit },
    MaxValue { value: FloatMajorUnit },
    PositiveOnly,
}
