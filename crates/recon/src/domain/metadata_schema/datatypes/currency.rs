//! Currency datatype validation implementations
//!
//! validate_value applies transformations then parses to Currency.

use common_enums::Currency;
use common_types::metadata_schema::datatypes::currency::{
    CurrencyConfig, CurrencyConfigV1, CurrencyTransformationRule, CurrencyTransformationRuleV1,
};
use error_stack::Report;

use crate::domain::metadata_schema::{
    config_validator::ConfigValidator,
    data_validator::{ValueTransformer, ValueValidator},
    transformations,
    types::{ConfigFieldValidationError, RuleError, ValueError},
};

impl ValueValidator for CurrencyConfig {
    type DataType = Currency;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(currency_config_v1) => currency_config_v1.validate_value(value),
        }
    }
}

impl ValueValidator for CurrencyConfigV1 {
    type DataType = Currency;
    fn validate_value(
        &self,
        mut value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        for transformation in &self.transformation_rules {
            transformation
                .transform_value(&mut value)
                .map_err(|e| ValueError::from_rule_error(e, value.clone()))?;
        }

        value
            .map(|currency| {
                currency.parse::<Currency>().map_err(|e| {
                    Report::new(ValueError::InvalidCurrency {
                        actual_value: Some(currency),
                        parse_error: e.to_string(),
                    })
                })
            })
            .transpose()
    }
}

impl ValueTransformer for CurrencyTransformationRule {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(rule_v1) => rule_v1.transform_value(value),
        }
    }
}

impl ValueTransformer for CurrencyTransformationRuleV1 {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::DefaultValue { currency } => {
                match value {
                    Some(_inner) => {}
                    _ => {
                        // None value → apply default
                        *value = Some(currency.to_string());
                    }
                }
            }
            Self::Trim => {
                if let Some(inner) = value {
                    *value = Some(inner.trim().to_string());
                }
            }
            Self::JsonExtract { pointer } => {
                return transformations::json_extract::json_extract_value(value, pointer);
            }
        }
        Ok(())
    }
}

// Currency config validators
impl ConfigValidator for CurrencyConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for CurrencyConfigV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use common_enums::Currency;
    use common_types::metadata_schema::datatypes::currency::{
        CurrencyConfigV1, CurrencyTransformationRule, CurrencyTransformationRuleV1,
    };

    use crate::domain::metadata_schema::data_validator::ValueValidator;

    fn default_value_rule(currency: Currency) -> CurrencyTransformationRule {
        CurrencyTransformationRule::V1(CurrencyTransformationRuleV1::DefaultValue { currency })
    }

    #[test]
    fn test_currency_default_value_not_applied_when_empty() {
        let config = CurrencyConfigV1 {
            transformation_rules: vec![default_value_rule(Currency::USD)],
        };
        let result = config.validate_value(Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn test_currency_default_value_applied_when_none() {
        let config = CurrencyConfigV1 {
            transformation_rules: vec![default_value_rule(Currency::USD)],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, Some(Currency::USD));
    }

    #[test]
    fn test_currency_valid_value_preserved() {
        let config = CurrencyConfigV1 {
            transformation_rules: vec![default_value_rule(Currency::USD)],
        };
        let result = config.validate_value(Some("EUR".to_string())).unwrap();
        assert_eq!(result, Some(Currency::EUR));
    }

    #[test]
    fn test_currency_invalid_value_fails() {
        let config = CurrencyConfigV1 {
            transformation_rules: vec![default_value_rule(Currency::USD)],
        };
        let result = config.validate_value(Some("INVALID".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_currency_no_default_empty_fails() {
        let config = CurrencyConfigV1 {
            transformation_rules: vec![],
        };
        let result = config.validate_value(Some(String::new()));
        assert!(result.is_err());
    }

    fn trim_rule() -> CurrencyTransformationRule {
        CurrencyTransformationRule::V1(CurrencyTransformationRuleV1::Trim)
    }

    #[test]
    fn test_currency_trim_before_parsing() {
        let config = CurrencyConfigV1 {
            transformation_rules: vec![trim_rule()],
        };
        let result = config.validate_value(Some("  USD  ".to_string())).unwrap();
        assert_eq!(result, Some(Currency::USD));
    }

    #[test]
    fn test_currency_without_trim_fails_on_padded_input() {
        let config = CurrencyConfigV1 {
            transformation_rules: vec![],
        };
        let result = config.validate_value(Some("  USD  ".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_currency_none_value() {
        let config = CurrencyConfigV1 {
            transformation_rules: vec![trim_rule()],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }
}
