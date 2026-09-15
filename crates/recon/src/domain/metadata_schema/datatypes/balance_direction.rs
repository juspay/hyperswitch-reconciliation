//! BalanceDirection datatype validation implementations
//!
//! validate_value applies transformations then matches against credit/debit values.

use common_enums::BalanceDirection;
use common_types::metadata_schema::datatypes::balance_direction::{
    BalanceDirectionConfig, BalanceDirectionConfigV1, BalanceDirectionTransformationRule,
    BalanceDirectionTransformationRuleV1,
};
use error_stack::{report, Report, ResultExt};
use std::collections::HashSet;

use crate::domain::metadata_schema::{
    config_validator::{validate_non_empty_string, ConfigValidator},
    data_validator::{ValueTransformer, ValueValidator},
    transformations,
    types::{ConfigFieldValidationError, RuleError, ValueError},
};

impl ValueValidator for BalanceDirectionConfig {
    type DataType = BalanceDirection;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(balance_config_v1) => balance_config_v1.validate_value(value),
        }
    }
}

impl ValueValidator for BalanceDirectionConfigV1 {
    type DataType = BalanceDirection;
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
            .map(|balance_direction| {
                if self.credit_values.contains(&balance_direction) {
                    Ok(BalanceDirection::Credit)
                } else if self.debit_values.contains(&balance_direction) {
                    Ok(BalanceDirection::Debit)
                } else {
                    Err(Report::new(ValueError::InvalidBalanceDirection {
                        actual_value: balance_direction,
                    }))
                }
            })
            .transpose()
    }
}

impl ValueTransformer for BalanceDirectionTransformationRule {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(rule_v1) => rule_v1.transform_value(value),
        }
    }
}

impl ValueTransformer for BalanceDirectionTransformationRuleV1 {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::DefaultValue { direction } => {
                match value {
                    Some(_inner) => {}
                    _ => {
                        // None value → apply default
                        *value = Some(direction.to_string());
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
            Self::StartsWith {
                prefix,
                then,
                otherwise,
            } => {
                if let Some(inner) = value {
                    *value = Some(if inner.starts_with(prefix.as_str()) {
                        then.clone()
                    } else {
                        otherwise.clone()
                    });
                }
            }
        }
        Ok(())
    }
}

// Balance direction config validators
impl ConfigValidator for BalanceDirectionConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for BalanceDirectionConfigV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        if self.credit_values.is_empty() || self.debit_values.is_empty() {
            return Err(ConfigFieldValidationError::EmptyBalanceDirectionValues.into());
        }

        self.credit_values
            .iter()
            .chain(&self.debit_values)
            .try_fold(
                HashSet::new(),
                |mut seen_balance_direction_values,
                 value|
                 -> Result<HashSet<String>, Report<Self::ErrorType>> {
                    validate_non_empty_string(value)
                        .change_context(ConfigFieldValidationError::EmptyBalanceDirectionValue)?;

                    if !seen_balance_direction_values.insert(value.clone()) {
                        return Err(
                            (ConfigFieldValidationError::DuplicateBalanceDirectionValue {
                                value: value.clone(),
                            })
                            .into(),
                        );
                    }

                    Ok(seen_balance_direction_values)
                },
            )?;

        self.transformation_rules
            .iter()
            .try_for_each(|rule| rule.validate_config())?;

        Ok(())
    }
}

impl ConfigValidator for BalanceDirectionTransformationRule {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(rule_v1) => rule_v1.validate_config(),
        }
    }
}

impl ConfigValidator for BalanceDirectionTransformationRuleV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::DefaultValue { .. } | Self::JsonExtract { .. } | Self::Trim => Ok(()),
            Self::StartsWith { prefix, .. } => {
                if prefix.is_empty() {
                    return Err(report!(
                        ConfigFieldValidationError::EmptyStartsWithRulePrefix
                    ));
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use common_enums::BalanceDirection;
    use common_types::metadata_schema::datatypes::balance_direction::{
        BalanceDirectionConfigV1, BalanceDirectionTransformationRule,
        BalanceDirectionTransformationRuleV1,
    };

    use crate::domain::metadata_schema::data_validator::ValueValidator;

    fn default_value_rule(direction: BalanceDirection) -> BalanceDirectionTransformationRule {
        BalanceDirectionTransformationRule::V1(BalanceDirectionTransformationRuleV1::DefaultValue {
            direction,
        })
    }

    fn test_config_with_default(direction: BalanceDirection) -> BalanceDirectionConfigV1 {
        BalanceDirectionConfigV1 {
            credit_values: vec!["credit".to_string()],
            debit_values: vec!["debit".to_string()],
            transformation_rules: vec![default_value_rule(direction)],
        }
    }

    fn test_config_no_default() -> BalanceDirectionConfigV1 {
        BalanceDirectionConfigV1 {
            credit_values: vec!["credit".to_string()],
            debit_values: vec!["debit".to_string()],
            transformation_rules: vec![],
        }
    }

    #[test]
    fn test_balance_direction_default_value_not_applied_when_empty() {
        let config = test_config_with_default(BalanceDirection::Credit);
        let result = config.validate_value(Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn test_balance_direction_default_value_applied_when_none() {
        let config = test_config_with_default(BalanceDirection::Credit);
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, Some(BalanceDirection::Credit));
    }

    #[test]
    fn test_balance_direction_credit_value_preserved() {
        let config = test_config_with_default(BalanceDirection::Debit);
        let result = config.validate_value(Some("credit".to_string())).unwrap();
        assert_eq!(result, Some(BalanceDirection::Credit));
    }

    #[test]
    fn test_balance_direction_invalid_value_fails() {
        let config = test_config_with_default(BalanceDirection::Credit);
        let result = config.validate_value(Some("invalid".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_balance_direction_no_default_empty_fails() {
        let config = test_config_no_default();
        let result = config.validate_value(Some(String::new()));
        assert!(result.is_err());
    }

    fn trim_rule() -> BalanceDirectionTransformationRule {
        BalanceDirectionTransformationRule::V1(BalanceDirectionTransformationRuleV1::Trim)
    }

    #[test]
    fn test_balance_direction_trim_before_matching() {
        let config = BalanceDirectionConfigV1 {
            credit_values: vec!["credit".to_string()],
            debit_values: vec!["debit".to_string()],
            transformation_rules: vec![trim_rule()],
        };
        let result = config
            .validate_value(Some("  credit  ".to_string()))
            .unwrap();
        assert_eq!(result, Some(BalanceDirection::Credit));
    }

    #[test]
    fn test_balance_direction_without_trim_fails_on_padded_input() {
        let config = BalanceDirectionConfigV1 {
            credit_values: vec!["credit".to_string()],
            debit_values: vec!["debit".to_string()],
            transformation_rules: vec![],
        };
        let result = config.validate_value(Some("  credit  ".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_balance_direction_none_value() {
        let config = BalanceDirectionConfigV1 {
            credit_values: vec!["credit".to_string()],
            debit_values: vec!["debit".to_string()],
            transformation_rules: vec![trim_rule()],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }

    fn starts_with_rule(
        prefix: &str,
        then: &str,
        otherwise: &str,
    ) -> BalanceDirectionTransformationRule {
        BalanceDirectionTransformationRule::V1(BalanceDirectionTransformationRuleV1::StartsWith {
            prefix: prefix.to_string(),
            then: then.to_string(),
            otherwise: otherwise.to_string(),
        })
    }

    fn credit_debit_config(
        rules: Vec<BalanceDirectionTransformationRule>,
    ) -> BalanceDirectionConfigV1 {
        BalanceDirectionConfigV1 {
            credit_values: vec!["credit".to_string()],
            debit_values: vec!["debit".to_string()],
            transformation_rules: rules,
        }
    }

    #[test]
    fn test_balance_direction_starts_with_uses_then_when_prefix_matches() {
        let config = credit_debit_config(vec![starts_with_rule("CR", "credit", "debit")]);
        let result = config.validate_value(Some("CR-12345".to_string())).unwrap();
        assert_eq!(result, Some(BalanceDirection::Credit));
    }

    #[test]
    fn test_balance_direction_starts_with_uses_otherwise_when_prefix_absent() {
        let config = credit_debit_config(vec![starts_with_rule("CR", "credit", "debit")]);
        let result = config.validate_value(Some("DR-67890".to_string())).unwrap();
        assert_eq!(result, Some(BalanceDirection::Debit));
    }

    #[test]
    fn test_balance_direction_starts_with_empty_value_takes_otherwise_branch() {
        let config = credit_debit_config(vec![starts_with_rule("CR", "credit", "debit")]);
        let result = config.validate_value(Some(String::new())).unwrap();
        assert_eq!(result, Some(BalanceDirection::Debit));
    }

    #[test]
    fn test_balance_direction_starts_with_none_value_passes_through() {
        let config = credit_debit_config(vec![starts_with_rule("CR", "credit", "debit")]);
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_balance_direction_starts_with_then_and_otherwise_can_swap_routing() {
        // prefix `CR` routes to `debit` and everything else to `credit` — confirms
        // the rule simply substitutes the configured strings with no implicit polarity.
        let config = credit_debit_config(vec![starts_with_rule("CR", "debit", "credit")]);
        assert_eq!(
            config.validate_value(Some("CR-12345".to_string())).unwrap(),
            Some(BalanceDirection::Debit)
        );
        assert_eq!(
            config.validate_value(Some("DR-67890".to_string())).unwrap(),
            Some(BalanceDirection::Credit)
        );
    }

    #[test]
    fn test_balance_direction_starts_with_empty_prefix_rejected_at_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = credit_debit_config(vec![starts_with_rule("", "credit", "debit")]);
        assert!(config.validate_config().is_err());
    }

    #[test]
    fn test_balance_direction_starts_with_non_empty_prefix_passes_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = credit_debit_config(vec![starts_with_rule("CR", "credit", "debit")]);
        assert!(config.validate_config().is_ok());
    }
}
