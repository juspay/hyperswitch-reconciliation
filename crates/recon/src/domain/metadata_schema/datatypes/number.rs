//! Number datatype validation implementations
//!
//! This module contains the ValueValidator and RuleValidator implementations
//! for f64 number types, including range validation rules.

use std::collections::HashSet;

use common_types::metadata_schema::{
    datatypes::number::{
        NumberConfig, NumberConfigV1, NumberTransformationRule, NumberTransformationRuleV1,
        NumberValidationRule, NumberValidationRuleV1,
    },
    RuleType,
};
use error_stack::{Report, ResultExt};

use crate::domain::metadata_schema::{
    config_validator::{validate_number, ConfigValidator, ToRuleType},
    data_validator::{RuleValidator, ValueTransformer, ValueValidator},
    transformations,
    types::{ConfigFieldValidationError, RuleError, ValueError},
};

impl ValueValidator for NumberConfig {
    type DataType = f64;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(number_config_v1) => number_config_v1.validate_value(value),
        }
    }
}

impl ValueValidator for NumberConfigV1 {
    type DataType = f64;
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
            .map(|value_str| {
                let number_value =
                    value_str
                        .parse::<f64>()
                        .map_err(|_| ValueError::InvalidType {
                            actual_value: value_str.clone(),
                        })?;

                for validation_rule in &self.validation_rules {
                    validation_rule
                        .validate_rule(&Some(number_value))
                        .map_err(|rule_error| {
                            ValueError::from_rule_error(rule_error, Some(number_value.to_string()))
                        })?;
                }

                Ok(number_value)
            })
            .transpose()
    }
}

impl RuleValidator for NumberValidationRule {
    type DataType = f64;
    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>> {
        match self {
            Self::V1(validation_rule_v1) => validation_rule_v1.validate_rule(value),
        }
    }
}

impl RuleValidator for NumberValidationRuleV1 {
    type DataType = f64;
    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>> {
        match self {
            Self::MinValue { value: min_value } => (*value)
                .map(|number_value| {
                    if number_value < *min_value {
                        Err(RuleError::ValueTooSmall {
                            min_value: min_value.to_string(),
                        }
                        .into())
                    } else {
                        Ok(number_value)
                    }
                })
                .transpose(),
            Self::MaxValue { value: max_value } => (*value)
                .map(|number_value| {
                    if number_value > *max_value {
                        Err(RuleError::ValueTooLarge {
                            max_value: max_value.to_string(),
                        }
                        .into())
                    } else {
                        Ok(number_value)
                    }
                })
                .transpose(),
        }
    }
}

impl ValueTransformer for NumberTransformationRule {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(rule_v1) => rule_v1.transform_value(value),
        }
    }
}

impl ValueTransformer for NumberTransformationRuleV1 {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
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

// Number config validators
impl ConfigValidator for NumberConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for NumberConfigV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        self.validation_rules.iter().try_fold(
            HashSet::new(),
            |mut seen_rules, rule| -> Result<HashSet<RuleType>, Report<Self::ErrorType>> {
                let rule_type = rule.to_rule_type();

                if !seen_rules.insert(rule_type.clone()) {
                    return Err(
                        (ConfigFieldValidationError::MultipleRulesOfSameTypeNotAllowed {
                            rule_type,
                        })
                        .into(),
                    );
                }

                rule.validate_config()?;

                Ok(seen_rules)
            },
        )?;

        Ok(())
    }
}

impl ConfigValidator for NumberValidationRule {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(rule_v1) => rule_v1.validate_config(),
        }
    }
}

impl ConfigValidator for NumberValidationRuleV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::MinValue { value } | Self::MaxValue { value } => validate_number(*value)
                .change_context(ConfigFieldValidationError::InvalidRuleSyntax {
                    rule_type: self.to_rule_type(),
                    details: "Value cannot be NaN or infinite".to_string(),
                }),
        }
    }
}

impl ToRuleType for NumberValidationRule {
    fn to_rule_type(&self) -> RuleType {
        match self {
            Self::V1(rule) => rule.to_rule_type(),
        }
    }
}

impl ToRuleType for NumberValidationRuleV1 {
    fn to_rule_type(&self) -> RuleType {
        match self {
            Self::MinValue { .. } => RuleType::MinValue,
            Self::MaxValue { .. } => RuleType::MaxValue,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use common_types::metadata_schema::datatypes::number::{
        NumberConfigV1, NumberTransformationRule, NumberTransformationRuleV1,
    };

    use crate::domain::metadata_schema::data_validator::ValueValidator;

    fn trim_rule() -> NumberTransformationRule {
        NumberTransformationRule::V1(NumberTransformationRuleV1::Trim)
    }

    #[test]
    fn test_number_trim_before_parsing() {
        let config = NumberConfigV1 {
            validation_rules: vec![],
            transformation_rules: vec![trim_rule()],
        };
        let result = config
            .validate_value(Some("  123.45  ".to_string()))
            .unwrap();
        assert_eq!(result, Some(123.45));
    }

    #[test]
    fn test_number_without_trim_fails_on_padded_input() {
        let config = NumberConfigV1 {
            validation_rules: vec![],
            transformation_rules: vec![],
        };
        let result = config.validate_value(Some("  123.45  ".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_number_none_value() {
        let config = NumberConfigV1 {
            validation_rules: vec![],
            transformation_rules: vec![trim_rule()],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }
}
