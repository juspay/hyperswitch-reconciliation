use common_enums::BalanceDirection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "balance_direction_config_version", rename_all = "snake_case")]
pub enum BalanceDirectionConfig {
    V1(BalanceDirectionConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BalanceDirectionConfigV1 {
    pub credit_values: Vec<String>,
    pub debit_values: Vec<String>,
    #[serde(default)]
    pub transformation_rules: Vec<BalanceDirectionTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(
    tag = "balance_direction_transformation_rule_version",
    rename_all = "snake_case"
)]
pub enum BalanceDirectionTransformationRule {
    V1(BalanceDirectionTransformationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(
    tag = "balance_direction_transformation_rule_type",
    rename_all = "snake_case"
)]
pub enum BalanceDirectionTransformationRuleV1 {
    DefaultValue {
        direction: BalanceDirection,
    },
    Trim,
    JsonExtract {
        pointer: String,
    },
    StartsWith {
        prefix: String,
        then: String,
        otherwise: String,
    },
}
