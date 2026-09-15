use std::collections::HashSet;

use common_types::{
    metadata_schema::{
        datatypes::majorunit::{
            MajorUnitConfig, MajorUnitConfigV1, MajorUnitTransformationRule,
            MajorUnitTransformationRuleV1, MajorUnitValidationRule, MajorUnitValidationRuleV1,
        },
        RuleType,
    },
    units::FloatMajorUnit,
};
use error_stack::Report;

use crate::domain::metadata_schema::{
    config_validator::ToRuleType,
    data_validator::{RuleValidator, ValueTransformer, ValueValidator},
    transformations,
    types::{ConfigFieldValidationError, RuleError},
    ConfigValidator, ValueError,
};

impl ValueValidator for MajorUnitConfig {
    type DataType = FloatMajorUnit;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(major_unit_config_v1) => major_unit_config_v1.validate_value(value),
        }
    }
}

impl ValueValidator for MajorUnitConfigV1 {
    type DataType = FloatMajorUnit;
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
                let normalized_value = value_str.replace(&self.delimiter.to_string(), ".");

                let major_unit_value =
                    normalized_value
                        .parse::<f64>()
                        .map_err(|_| ValueError::InvalidType {
                            actual_value: value_str,
                        })?;

                let major_unit = FloatMajorUnit::new(major_unit_value);

                for validation_rule in &self.validation_rules {
                    validation_rule
                        .validate_rule(&Some(major_unit))
                        .map_err(|rule_error| {
                            ValueError::from_rule_error(rule_error, Some(major_unit.to_string()))
                        })?;
                }

                Ok(major_unit)
            })
            .transpose()
    }
}

impl RuleValidator for MajorUnitValidationRule {
    type DataType = FloatMajorUnit;
    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>> {
        match self {
            Self::V1(validation_rule_v1) => validation_rule_v1.validate_rule(value),
        }
    }
}

impl RuleValidator for MajorUnitValidationRuleV1 {
    type DataType = FloatMajorUnit;
    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>> {
        match self {
            Self::MinValue { value: min_value } => (*value)
                .map(|major_unit| {
                    if major_unit < *min_value {
                        Err(RuleError::ValueTooSmall {
                            min_value: min_value.to_string(),
                        }
                        .into())
                    } else {
                        Ok(major_unit)
                    }
                })
                .transpose(),
            Self::MaxValue { value: max_value } => (*value)
                .map(|major_unit| {
                    if major_unit > *max_value {
                        Err(RuleError::ValueTooLarge {
                            max_value: max_value.to_string(),
                        }
                        .into())
                    } else {
                        Ok(major_unit)
                    }
                })
                .transpose(),
            Self::PositiveOnly => (*value)
                .map(|major_unit| {
                    if major_unit.is_negative() {
                        Err(RuleError::NegativeValue.into())
                    } else {
                        Ok(major_unit)
                    }
                })
                .transpose(),
        }
    }
}

impl ValueTransformer for MajorUnitTransformationRule {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(rule_v1) => rule_v1.transform_value(value),
        }
    }
}

impl ValueTransformer for MajorUnitTransformationRuleV1 {
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
            Self::Negate => {
                if let Some(inner) = value {
                    if inner.is_empty() {
                    } else if let Some(stripped) = inner.strip_prefix('-') {
                        *value = Some(stripped.to_string());
                    } else {
                        *value = Some(format!("-{}", inner));
                    }
                }
            }
            Self::Absolute => {
                if let Some(inner) = value {
                    if let Some(stripped) = inner.strip_prefix('-') {
                        *value = Some(stripped.to_string());
                    }
                }
            }
            Self::ReplaceChar { from, to, mode } => {
                transformations::replace_char::replace_char_in_value(value, *from, *to, mode);
            }
        }
        Ok(())
    }
}

// FloatMajorUnit config validators
impl ConfigValidator for MajorUnitConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for MajorUnitConfigV1 {
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

impl ConfigValidator for MajorUnitValidationRule {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(rule_v1) => rule_v1.validate_config(),
        }
    }
}

impl ConfigValidator for MajorUnitValidationRuleV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::MinValue { .. } | Self::MaxValue { .. } | Self::PositiveOnly => {
                // Basic validation - FloatMajorUnit type ensures valid values
                Ok(())
            }
        }
    }
}

impl ToRuleType for MajorUnitValidationRule {
    fn to_rule_type(&self) -> RuleType {
        match self {
            Self::V1(rule) => rule.to_rule_type(),
        }
    }
}

impl ToRuleType for MajorUnitValidationRuleV1 {
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
    use common_types::metadata_schema::datatypes::majorunit::{
        AmountDelimiter, MajorUnitConfigV1, MajorUnitTransformationRule,
        MajorUnitTransformationRuleV1, MajorUnitValidationRule, MajorUnitValidationRuleV1,
        ReplaceMode,
    };
    use common_types::units::FloatMajorUnit;

    use crate::domain::metadata_schema::data_validator::ValueValidator;

    fn trim_rule() -> MajorUnitTransformationRule {
        MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::Trim)
    }

    fn negate_rule() -> MajorUnitTransformationRule {
        MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::Negate)
    }

    fn absolute_rule() -> MajorUnitTransformationRule {
        MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::Absolute)
    }

    fn positive_only_rule() -> MajorUnitValidationRule {
        MajorUnitValidationRule::V1(MajorUnitValidationRuleV1::PositiveOnly)
    }

    #[test]
    fn test_major_unit_trim_before_parsing() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![],
            transformation_rules: vec![trim_rule()],
        };
        let result = config
            .validate_value(Some("  123.45  ".to_string()))
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_major_unit_without_trim_fails_on_padded_input() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![],
            transformation_rules: vec![],
        };
        let result = config.validate_value(Some("  123.45  ".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_major_unit_none_value() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![],
            transformation_rules: vec![trim_rule()],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_major_unit_negate_flips_sign() {
        // Core negate behavior: "-2750.00" → 2750.00, and "2750.00" → -2750.00.
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![],
            transformation_rules: vec![trim_rule(), negate_rule()],
        };
        assert_eq!(
            config.validate_value(Some("-2750.00".to_string())).unwrap(),
            Some(FloatMajorUnit::new(2750.00))
        );
        assert_eq!(
            config.validate_value(Some("2750.00".to_string())).unwrap(),
            Some(FloatMajorUnit::new(-2750.00))
        );
    }

    #[test]
    fn test_major_unit_negate_with_positive_only() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![positive_only_rule()],
            transformation_rules: vec![trim_rule(), negate_rule()],
        };
        assert_eq!(
            config.validate_value(Some("-2750.00".to_string())).unwrap(),
            Some(FloatMajorUnit::new(2750.00))
        );
        assert!(config.validate_value(Some("2750.00".to_string())).is_err());
    }

    #[test]
    fn test_major_unit_absolute_strips_sign_only_when_negative() {
        // Core absolute behavior: "-2750.00" → 2750.00; "2750.00" stays 2750.00.
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![],
            transformation_rules: vec![trim_rule(), absolute_rule()],
        };
        assert_eq!(
            config.validate_value(Some("-2750.00".to_string())).unwrap(),
            Some(FloatMajorUnit::new(2750.00))
        );
        assert_eq!(
            config.validate_value(Some("2750.00".to_string())).unwrap(),
            Some(FloatMajorUnit::new(2750.00))
        );
    }

    #[test]
    fn test_major_unit_absolute_satisfies_positive_only() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![positive_only_rule()],
            transformation_rules: vec![trim_rule(), absolute_rule()],
        };
        assert_eq!(
            config.validate_value(Some("-2750.00".to_string())).unwrap(),
            Some(FloatMajorUnit::new(2750.00))
        );
        assert_eq!(
            config.validate_value(Some("2750.00".to_string())).unwrap(),
            Some(FloatMajorUnit::new(2750.00))
        );
    }

    fn replace_char_rule(
        from: char,
        to: Option<char>,
        mode: ReplaceMode,
    ) -> MajorUnitTransformationRule {
        MajorUnitTransformationRule::V1(MajorUnitTransformationRuleV1::ReplaceChar {
            from,
            to,
            mode,
        })
    }

    #[test]
    fn test_major_unit_replace_char_strips_us_grouping() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![],
            transformation_rules: vec![trim_rule(), replace_char_rule(',', None, ReplaceMode::All)],
        };
        assert_eq!(
            config
                .validate_value(Some("1,000,000.00".to_string()))
                .unwrap(),
            Some(FloatMajorUnit::new(1000000.00))
        );
    }

    #[test]
    fn test_major_unit_replace_char_strips_indian_grouping() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![],
            transformation_rules: vec![replace_char_rule(',', None, ReplaceMode::All)],
        };
        assert_eq!(
            config.validate_value(Some("1,00,000".to_string())).unwrap(),
            Some(FloatMajorUnit::new(100000.0))
        );
    }

    #[test]
    fn test_major_unit_replace_char_strips_eu_grouping_with_delimiter() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Comma,
            validation_rules: vec![],
            transformation_rules: vec![replace_char_rule('.', None, ReplaceMode::All)],
        };
        assert_eq!(
            config
                .validate_value(Some("1.000.000,00".to_string()))
                .unwrap(),
            Some(FloatMajorUnit::new(1000000.00))
        );
    }

    #[test]
    fn test_major_unit_replace_char_single_last_then_all_same_char() {
        let config = MajorUnitConfigV1 {
            delimiter: AmountDelimiter::Dot,
            validation_rules: vec![],
            transformation_rules: vec![
                replace_char_rule(
                    ',',
                    Some('.'),
                    ReplaceMode::Single {
                        occurrence: 0,
                        from_end: true,
                    },
                ),
                replace_char_rule(',', None, ReplaceMode::All),
            ],
        };
        assert_eq!(
            config.validate_value(Some("1,234,56".to_string())).unwrap(),
            Some(FloatMajorUnit::new(1234.56))
        );
    }
}
