use common_enums::Currency;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "currency_config_version", rename_all = "snake_case")]
pub enum CurrencyConfig {
    V1(CurrencyConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CurrencyConfigV1 {
    #[serde(default)]
    pub transformation_rules: Vec<CurrencyTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(
    tag = "currency_transformation_rule_version",
    rename_all = "snake_case"
)]
pub enum CurrencyTransformationRule {
    V1(CurrencyTransformationRuleV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "currency_transformation_rule_type", rename_all = "snake_case")]
pub enum CurrencyTransformationRuleV1 {
    DefaultValue { currency: Currency },
    Trim,
    JsonExtract { pointer: String },
}
