//! MinorUnit datatype validation implementations
//!
//! This module contains the ValueValidator and RuleValidator implementations
//! for MinorUnit types, including range validation rules.

use std::collections::HashSet;

use common_types::{
    metadata_schema::{
        datatypes::minorunit::{
            MinorUnitConfig, MinorUnitConfigV1, MinorUnitTransformationRule,
            MinorUnitTransformationRuleV1, MinorUnitValidationRule, MinorUnitValidationRuleV1,
        },
        RuleType,
    },
    units::MinorUnit,
};
use error_stack::Report;

use crate::domain::metadata_schema::{
    config_validator::{ConfigValidator, ToRuleType},
    data_validator::{RuleValidator, ValueTransformer, ValueValidator},
    transformations,
    types::{ConfigFieldValidationError, RuleError, ValueError},
};

impl ValueValidator for MinorUnitConfig {
    type DataType = MinorUnit;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(minor_unit_config_v1) => minor_unit_config_v1.validate_value(value),
        }
    }
}

impl ValueValidator for MinorUnitConfigV1 {
    type DataType = MinorUnit;

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
                let minor_unit_value =
                    value_str
                        .parse::<i64>()
                        .map_err(|_| ValueError::InvalidType {
                            actual_value: value_str.clone(),
                        })?;

                let minor_unit = MinorUnit::new(minor_unit_value);

                for validation_rule in &self.validation_rules {
                    validation_rule
                        .validate_rule(&Some(minor_unit))
                        .map_err(|rule_error| {
                            ValueError::from_rule_error(
                                rule_error,
                                Some(minor_unit.get_amount_as_i64().to_string()),
                            )
                        })?;
                }

                Ok(minor_unit)
            })
            .transpose()
    }
}

impl RuleValidator for MinorUnitValidationRule {
    type DataType = MinorUnit;
    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>> {
        match self {
            Self::V1(validation_rule_v1) => validation_rule_v1.validate_rule(value),
        }
    }
}

impl RuleValidator for MinorUnitValidationRuleV1 {
    type DataType = MinorUnit;

    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>> {
        match self {
            Self::MinValue { value: min_value } => (*value)
                .map(|minor_unit| {
                    if minor_unit < *min_value {
                        Err(RuleError::ValueTooSmall {
                            min_value: min_value.get_amount_as_i64().to_string(),
                        }
                        .into())
                    } else {
                        Ok(minor_unit)
                    }
                })
                .transpose(),
            Self::MaxValue { value: max_value } => (*value)
                .map(|minor_unit| {
                    if minor_unit > *max_value {
                        Err(RuleError::ValueTooLarge {
                            max_value: max_value.get_amount_as_i64().to_string(),
                        }
                        .into())
                    } else {
                        Ok(minor_unit)
                    }
                })
                .transpose(),
            Self::PositiveOnly => (*value)
                .map(|minor_unit| {
                    if minor_unit.get_amount_as_i64() < 0 {
                        Err(RuleError::NegativeValue.into())
                    } else {
                        Ok(minor_unit)
                    }
                })
                .transpose(),
        }
    }
}

impl ValueTransformer for MinorUnitTransformationRule {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(rule_v1) => rule_v1.transform_value(value),
        }
    }
}

impl ValueTransformer for MinorUnitTransformationRuleV1 {
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
            Self::Absolute => {
                if let Some(inner) = value {
                    if let Some(stripped) = inner.strip_prefix('-') {
                        *value = Some(stripped.to_string());
                    }
                }
            }
        }
        Ok(())
    }
}

// MinorUnit config validators
impl ConfigValidator for MinorUnitConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for MinorUnitConfigV1 {
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

impl ConfigValidator for MinorUnitValidationRule {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(rule_v1) => rule_v1.validate_config(),
        }
    }
}

impl ConfigValidator for MinorUnitValidationRuleV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::MinValue { .. } | Self::MaxValue { .. } | Self::PositiveOnly => {
                // Basic validation - MinorUnit type ensures valid values
                Ok(())
            }
        }
    }
}

impl ToRuleType for MinorUnitValidationRule {
    fn to_rule_type(&self) -> RuleType {
        match self {
            Self::V1(rule) => rule.to_rule_type(),
        }
    }
}

impl ToRuleType for MinorUnitValidationRuleV1 {
    fn to_rule_type(&self) -> RuleType {
        match self {
            Self::MinValue { .. } => RuleType::MinValue,
            Self::MaxValue { .. } => RuleType::MaxValue,
            Self::PositiveOnly => RuleType::PositiveOnly,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use common_types::{
        metadata_schema::datatypes::minorunit::{
            MinorUnitConfigV1, MinorUnitTransformationRule, MinorUnitTransformationRuleV1,
            MinorUnitValidationRule, MinorUnitValidationRuleV1,
        },
        units::MinorUnit,
    };

    use crate::domain::metadata_schema::data_validator::ValueValidator;

    fn trim_rule() -> MinorUnitTransformationRule {
        MinorUnitTransformationRule::V1(MinorUnitTransformationRuleV1::Trim)
    }

    fn absolute_rule() -> MinorUnitTransformationRule {
        MinorUnitTransformationRule::V1(MinorUnitTransformationRuleV1::Absolute)
    }

    fn positive_only_rule() -> MinorUnitValidationRule {
        MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)
    }

    #[test]
    fn test_minor_unit_trim_before_parsing() {
        let config = MinorUnitConfigV1 {
            validation_rules: vec![],
            transformation_rules: vec![trim_rule()],
        };
        let result = config
            .validate_value(Some("  12345  ".to_string()))
            .unwrap();
        assert_eq!(result, Some(MinorUnit::new(12345)));
    }

    #[test]
    fn test_minor_unit_without_trim_fails_on_padded_input() {
        let config = MinorUnitConfigV1 {
            validation_rules: vec![],
            transformation_rules: vec![],
        };
        let result = config.validate_value(Some("  12345  ".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_minor_unit_none_value() {
        let config = MinorUnitConfigV1 {
            validation_rules: vec![],
            transformation_rules: vec![trim_rule()],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_minor_unit_json_extract_then_parse() {
        let config = MinorUnitConfigV1 {
            validation_rules: vec![],
            transformation_rules: vec![MinorUnitTransformationRule::V1(
                MinorUnitTransformationRuleV1::JsonExtract {
                    pointer: "/amount".to_string(),
                },
            )],
        };
        let input = r#"{"amount": "12345"}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some(MinorUnit::new(12345)));
    }

    #[test]
    fn test_minor_unit_absolute_strips_sign_only_when_negative() {
        // Core absolute behavior: "-12345" → 12345; "12345" stays "12345".
        let config = MinorUnitConfigV1 {
            validation_rules: vec![],
            transformation_rules: vec![trim_rule(), absolute_rule()],
        };
        assert_eq!(
            config.validate_value(Some("-12345".to_string())).unwrap(),
            Some(MinorUnit::new(12345))
        );
        assert_eq!(
            config.validate_value(Some("12345".to_string())).unwrap(),
            Some(MinorUnit::new(12345))
        );
    }

    #[test]
    fn test_minor_unit_absolute_satisfies_positive_only() {
        let config = MinorUnitConfigV1 {
            validation_rules: vec![positive_only_rule()],
            transformation_rules: vec![trim_rule(), absolute_rule()],
        };
        assert_eq!(
            config.validate_value(Some("-12345".to_string())).unwrap(),
            Some(MinorUnit::new(12345))
        );
        assert_eq!(
            config.validate_value(Some("12345".to_string())).unwrap(),
            Some(MinorUnit::new(12345))
        );
    }
}
