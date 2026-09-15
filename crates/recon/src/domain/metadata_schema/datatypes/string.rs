//! String datatype validation implementations
//!
//! This module contains the ValueValidator and RuleValidator implementations
//! for String types, including length validation rules.

use std::collections::HashSet;

use common_types::metadata_schema::{
    datatypes::string::{
        StringConfig, StringConfigV1, StringTransformationRule, StringTransformationRuleV1,
        StringValidationRule, StringValidationRuleV1,
    },
    RuleType,
};
use error_stack::Report;

use crate::domain::metadata_schema::{
    config_validator::{ConfigValidator, ToRuleType},
    data_validator::{RuleValidator, ValueTransformer, ValueValidator},
    transformations::{self, regex_extract::validate_regex_config},
    types::{ConfigFieldValidationError, RuleError, ValueError},
};

impl ValueValidator for StringConfig {
    type DataType = String;

    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(string_config_v1) => string_config_v1.validate_value(value),
        }
    }
}

impl ValueValidator for StringConfigV1 {
    type DataType = String;

    fn validate_value(
        &self,
        mut value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        for transformation in &self.transformation_rules {
            transformation
                .transform_value(&mut value)
                .map_err(|transformation_error| {
                    ValueError::from_rule_error(transformation_error, value.clone())
                })?;
        }

        for validation_rule in &self.validation_rules {
            validation_rule
                .validate_rule(&value)
                .map_err(|rule_error| ValueError::from_rule_error(rule_error, value.clone()))?;
        }

        Ok(value)
    }
}

impl RuleValidator for StringValidationRule {
    type DataType = String;
    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>> {
        match self {
            Self::V1(validation_rule_v1) => validation_rule_v1.validate_rule(value),
        }
    }
}

impl RuleValidator for StringValidationRuleV1 {
    type DataType = String;
    fn validate_rule(
        &self,
        value: &Option<Self::DataType>,
    ) -> Result<Option<Self::DataType>, Report<RuleError>> {
        match self {
            Self::MinLength { value: min_length } => value
                .as_ref()
                .map(|inner_value| {
                    if inner_value.len() < *min_length {
                        Err(RuleError::ValueTooShort {
                            min_length: min_length.to_string(),
                        }
                        .into())
                    } else {
                        Ok(inner_value.clone())
                    }
                })
                .transpose(),
            Self::MaxLength { value: max_length } => value
                .as_ref()
                .map(|inner_value| {
                    if inner_value.len() > *max_length {
                        Err(RuleError::ValueTooLong {
                            max_length: max_length.to_string(),
                        }
                        .into())
                    } else {
                        Ok(inner_value.clone())
                    }
                })
                .transpose(),
        }
    }
}

impl ValueTransformer for StringTransformationRule {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(transformation_rule_v1) => transformation_rule_v1.transform_value(value),
        }
    }
}

impl ValueTransformer for StringTransformationRuleV1 {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        let new_string = match self {
            Self::DefaultValue { value: default } => match value {
                Some(inner) => Some(inner.clone()),
                None => Some(default.clone()),
            },
            Self::ToUpperCase => value.as_ref().map(|inner| inner.to_uppercase()),
            Self::ToLowerCase => value.as_ref().map(|inner| inner.to_lowercase()),
            Self::Trim => value.as_ref().map(|inner| inner.trim().to_string()),
            Self::StripPrefix { prefix } => value
                .as_ref()
                .map(|inner| {
                    inner
                        .strip_prefix(prefix)
                        .ok_or(RuleError::PrefixNotFound {
                            prefix: prefix.clone(),
                        })
                        .map(|s| s.to_string())
                })
                .transpose()?,
            Self::StripSuffix { suffix } => value
                .as_ref()
                .map(|inner| {
                    inner
                        .strip_suffix(suffix)
                        .ok_or(RuleError::SuffixNotFound {
                            suffix: suffix.clone(),
                        })
                        .map(|s| s.to_string())
                })
                .transpose()?,
            Self::JsonExtract { pointer } => {
                return transformations::json_extract::json_extract_value(value, pointer);
            }
            Self::Regex { pattern, group } => {
                return transformations::regex_extract::regex_extract_value(value, pattern, *group);
            }
            Self::ReplaceChar { from, to, mode } => {
                transformations::replace_char::replace_char_in_value(value, *from, *to, mode);
                return Ok(());
            }
        };

        *value = new_string;
        Ok(())
    }
}

impl ConfigValidator for StringConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(string_config_v1) => string_config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for StringConfigV1 {
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

        self.transformation_rules
            .iter()
            .try_for_each(|rule| rule.validate_config())?;

        Ok(())
    }
}

impl ConfigValidator for StringValidationRule {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(rule) => rule.validate_config(),
        }
    }
}

impl ConfigValidator for StringValidationRuleV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::MaxLength { value } => {
                if *value == 0 {
                    return Err((ConfigFieldValidationError::InvalidRuleSyntax {
                        rule_type: self.to_rule_type(),
                        details: "MaxLength must be greater than 0".to_string(),
                    })
                    .into());
                }
                Ok(())
            }
            Self::MinLength { value: _ } => Ok(()),
        }
    }
}

impl ConfigValidator for StringTransformationRule {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(rule) => rule.validate_config(),
        }
    }
}

impl ConfigValidator for StringTransformationRuleV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::DefaultValue { .. }
            | Self::ToUpperCase
            | Self::ToLowerCase
            | Self::StripPrefix { .. }
            | Self::StripSuffix { .. }
            | Self::Trim
            | Self::JsonExtract { .. }
            | Self::ReplaceChar { .. } => Ok(()),
            Self::Regex { pattern, group } => validate_regex_config(pattern, *group),
        }
    }
}

impl ToRuleType for StringValidationRule {
    fn to_rule_type(&self) -> RuleType {
        match self {
            Self::V1(rule) => rule.to_rule_type(),
        }
    }
}

impl ToRuleType for StringValidationRuleV1 {
    fn to_rule_type(&self) -> RuleType {
        match self {
            Self::MaxLength { .. } => RuleType::MaxLength,
            Self::MinLength { .. } => RuleType::MinLength,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::domain::metadata_schema::{
        config_validator::ConfigValidator, data_validator::ValueValidator,
    };
    use common_types::metadata_schema::datatypes::{
        majorunit::ReplaceMode,
        string::{
            StringConfigV1, StringTransformationRule, StringTransformationRuleV1,
            StringValidationRule, StringValidationRuleV1,
        },
    };

    fn trim_rule() -> StringTransformationRule {
        StringTransformationRule::V1(StringTransformationRuleV1::Trim)
    }

    fn default_value_rule(value: &str) -> StringTransformationRule {
        StringTransformationRule::V1(StringTransformationRuleV1::DefaultValue {
            value: value.to_string(),
        })
    }

    #[test]
    fn test_string_default_value_applied_when_none() {
        let config = StringConfigV1 {
            transformation_rules: vec![default_value_rule("fallback")],
            validation_rules: vec![],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, Some("fallback".to_string()));
    }

    #[test]
    fn test_string_default_value_not_applied_when_empty() {
        let config = StringConfigV1 {
            transformation_rules: vec![default_value_rule("fallback")],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some(String::new())).unwrap();
        assert_eq!(result, Some(String::new()));
    }

    #[test]
    fn test_string_default_value_preserves_non_empty() {
        let config = StringConfigV1 {
            transformation_rules: vec![default_value_rule("fallback")],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("actual".to_string())).unwrap();
        assert_eq!(result, Some("actual".to_string()));
    }

    #[test]
    fn test_string_default_value_allows_empty_default() {
        let config = StringConfigV1 {
            transformation_rules: vec![default_value_rule("")],
            validation_rules: vec![],
        };
        config.validate_config().unwrap();
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, Some(String::new()));
    }

    #[test]
    fn test_string_default_value_with_min_length_validation() {
        let config = StringConfigV1 {
            transformation_rules: vec![default_value_rule("fallback")],
            validation_rules: vec![StringValidationRule::V1(
                StringValidationRuleV1::MinLength { value: 5 },
            )],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, Some("fallback".to_string()));
    }

    #[test]
    fn test_string_default_value_not_applied_to_empty_with_min_length_validation() {
        let config = StringConfigV1 {
            transformation_rules: vec![default_value_rule("fallback")],
            validation_rules: vec![StringValidationRule::V1(
                StringValidationRuleV1::MinLength { value: 1 },
            )],
        };
        let result = config.validate_value(Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn test_string_trim_leading_spaces() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule()],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("  hello".to_string())).unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_string_trim_trailing_spaces() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule()],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("hello  ".to_string())).unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_string_trim_both_sides() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule()],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("  hello  ".to_string()))
            .unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_string_trim_only_spaces() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule()],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("   ".to_string())).unwrap();
        assert_eq!(result, Some("".to_string()));
    }

    #[test]
    fn test_string_trim_with_tabs_and_newlines() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule()],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("\t\nhello\t\n".to_string()))
            .unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_string_trim_empty_string() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule()],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some(String::new())).unwrap();
        assert_eq!(result, Some("".to_string()));
    }

    #[test]
    fn test_string_trim_none_value() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule()],
            validation_rules: vec![],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_string_trim_with_length_validation() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule()],
            validation_rules: vec![StringValidationRule::V1(
                StringValidationRuleV1::MinLength { value: 5 },
            )],
        };
        // "  hello  " -> "hello" (5 chars) should pass
        let result = config
            .validate_value(Some("  hello  ".to_string()))
            .unwrap();
        assert_eq!(result, Some("hello".to_string()));

        // "  hi  " -> "hi" (2 chars) should fail min length
        let result = config.validate_value(Some("  hi  ".to_string()));
        assert!(result.is_err());
    }

    fn json_extract_rule(pointer: &str) -> StringTransformationRule {
        StringTransformationRule::V1(StringTransformationRuleV1::JsonExtract {
            pointer: pointer.to_string(),
        })
    }

    #[test]
    fn test_json_extract_settlement_batch() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/settlementBatch")],
            validation_rules: vec![],
        };
        let input = r#"{"pspReference":"SCJD7CBRBLHRRWX3","settlementBatch":"69"}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some("69".to_string()));
    }

    #[test]
    fn test_json_extract_missing_key() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/doesNotExist")],
            validation_rules: vec![],
        };
        let input = r#"{"pspReference":"SCJD7CBRBLHRRWX3"}"#;
        let result = config.validate_value(Some(input.to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_json_extract_invalid_json() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/key")],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("not json".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_json_extract_none_value() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/key")],
            validation_rules: vec![],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_json_extract_then_trim() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/name"), trim_rule()],
            validation_rules: vec![],
        };
        let input = r#"{"name":"  hello  "}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_json_extract_value_is_number() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/k")],
            validation_rules: vec![],
        };
        let input = r#"{"k": 69}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some("69".to_string()));
    }

    #[test]
    fn test_json_extract_value_is_bool() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/k")],
            validation_rules: vec![],
        };
        let input = r#"{"k": true}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some("true".to_string()));
    }

    #[test]
    fn test_json_extract_value_is_null() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/k")],
            validation_rules: vec![],
        };
        let input = r#"{"k": null}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_json_extract_value_is_object() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/k")],
            validation_rules: vec![],
        };
        let input = r#"{"k": {"nested": 1}}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some(r#"{"nested":1}"#.to_string()));
    }

    #[test]
    fn test_json_extract_nested_object() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/address/state")],
            validation_rules: vec![],
        };
        let input = r#"{"address":{"state":"Karnataka","city":"Bangalore"}}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some("Karnataka".to_string()));
    }

    #[test]
    fn test_json_extract_array_index() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/items/0")],
            validation_rules: vec![],
        };
        let input = r#"{"items":["apple","banana","cherry"]}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some("apple".to_string()));
    }

    #[test]
    fn test_json_extract_array_of_objects() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/users/0/name")],
            validation_rules: vec![],
        };
        let input = r#"{"users":[{"name":"John"},{"name":"Jane"}]}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some("John".to_string()));
    }

    #[test]
    fn test_json_extract_deep_mixed() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/data/txns/0/amt/val")],
            validation_rules: vec![],
        };
        let input = r#"{"data":{"txns":[{"amt":{"val":1234}}]}}"#;
        let result = config.validate_value(Some(input.to_string())).unwrap();
        assert_eq!(result, Some("1234".to_string()));
    }

    #[test]
    fn test_json_extract_array_out_of_bounds() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/items/99")],
            validation_rules: vec![],
        };
        let input = r#"{"items":["apple","banana"]}"#;
        let result = config.validate_value(Some(input.to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_json_extract_nested_key_missing() {
        let config = StringConfigV1 {
            transformation_rules: vec![json_extract_rule("/address/zipcode")],
            validation_rules: vec![],
        };
        let input = r#"{"address":{"state":"KA"}}"#;
        let result = config.validate_value(Some(input.to_string()));
        assert!(result.is_err());
    }

    fn regex_rule(pattern: &str) -> StringTransformationRule {
        StringTransformationRule::V1(StringTransformationRuleV1::Regex {
            pattern: pattern.to_string(),
            group: None,
        })
    }

    fn regex_rule_with_group(pattern: &str, group: usize) -> StringTransformationRule {
        StringTransformationRule::V1(StringTransformationRuleV1::Regex {
            pattern: pattern.to_string(),
            group: Some(group),
        })
    }

    #[test]
    fn test_regex_full_match_no_capture() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule(r"\d+")],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("abc123def".to_string()))
            .unwrap();
        assert_eq!(result, Some("123".to_string()));
    }

    #[test]
    fn test_regex_capture_group() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule_with_group(r"Order_ID:\s*(\d+)", 1)],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("Order_ID: 12345, date=2026-04-23".to_string()))
            .unwrap();
        assert_eq!(result, Some("12345".to_string()));
    }

    #[test]
    fn test_regex_first_capture_group_wins() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule_with_group(r"(\w+)=(\w+)", 1)],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("key=value".to_string()))
            .unwrap();
        assert_eq!(result, Some("key".to_string()));
    }

    #[test]
    fn test_regex_no_match_errors() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule(r"\d+")],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("no digits here".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_invalid_pattern_errors_at_runtime() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule("[")],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("anything".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_invalid_pattern_rejected_at_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule("[")],
            validation_rules: vec![],
        };
        assert!(config.validate_config().is_err());
    }

    #[test]
    fn test_regex_valid_pattern_passes_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule(r"\d+")],
            validation_rules: vec![],
        };
        assert!(config.validate_config().is_ok());
    }

    #[test]
    fn test_regex_none_value_passthrough() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule(r"\d+")],
            validation_rules: vec![],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_regex_then_trim() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule_with_group(r"\[(.+?)\]", 1), trim_rule()],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("prefix [  hello  ] suffix".to_string()))
            .unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_regex_unicode_input() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule(r"\p{Greek}+")],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("latin αβγ latin".to_string()))
            .unwrap();
        assert_eq!(result, Some("αβγ".to_string()));
    }

    #[test]
    fn test_regex_empty_string_input_no_match() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule(r"\d+")],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_anchored_pattern() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule_with_group(r"^txn_(\w+)$", 1)],
            validation_rules: vec![],
        };
        let ok = config
            .validate_value(Some("txn_abc123".to_string()))
            .unwrap();
        assert_eq!(ok, Some("abc123".to_string()));

        let err = config.validate_value(Some("prefix_txn_abc123".to_string()));
        assert!(err.is_err());
    }

    #[test]
    fn test_regex_default_group_returns_whole_match() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule(r"Order_ID:\s*(\d+)")],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("Order_ID: 12345, date=2026-04-23".to_string()))
            .unwrap();
        assert_eq!(result, Some("Order_ID: 12345".to_string()));
    }

    #[test]
    fn test_regex_group_second_capture() {
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule_with_group(r"(\w+)=(\w+)", 2)],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("key=value".to_string()))
            .unwrap();
        assert_eq!(result, Some("value".to_string()));
    }

    #[test]
    fn test_regex_group_out_of_range_rejected_at_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule_with_group(r"(\w+)=(\w+)", 9)],
            validation_rules: vec![],
        };
        assert!(config.validate_config().is_err());
    }

    #[test]
    fn test_regex_group_in_range_passes_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule_with_group(r"(\w+)=(\w+)", 2)],
            validation_rules: vec![],
        };
        assert!(config.validate_config().is_ok());
    }

    #[test]
    fn test_regex_group_zero_passes_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = StringConfigV1 {
            transformation_rules: vec![regex_rule_with_group(r"\w+", 0)],
            validation_rules: vec![],
        };
        assert!(config.validate_config().is_ok());
    }

    fn replace_char_rule(
        from: char,
        to: Option<char>,
        mode: ReplaceMode,
    ) -> StringTransformationRule {
        StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar { from, to, mode })
    }

    #[test]
    fn test_string_replace_char_all_deletes_every_occurrence() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule('-', None, ReplaceMode::All)],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("TXN-2026-00123".to_string()))
            .unwrap();
        assert_eq!(result, Some("TXN202600123".to_string()));
    }

    #[test]
    fn test_string_replace_char_all_replaces_every_occurrence() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule('/', Some('_'), ReplaceMode::All)],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("a/b/c".to_string())).unwrap();
        assert_eq!(result, Some("a_b_c".to_string()));
    }

    #[test]
    fn test_string_replace_char_single_from_start_targets_nth() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule(
                '-',
                Some('_'),
                ReplaceMode::Single {
                    occurrence: 1,
                    from_end: false,
                },
            )],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("a-b-c".to_string())).unwrap();
        assert_eq!(result, Some("a-b_c".to_string()));
    }

    #[test]
    fn test_string_replace_char_single_from_end_targets_last() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule(
                '/',
                Some('_'),
                ReplaceMode::Single {
                    occurrence: 0,
                    from_end: true,
                },
            )],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("a/b/c".to_string())).unwrap();
        assert_eq!(result, Some("a/b_c".to_string()));
    }

    #[test]
    fn test_string_replace_char_single_out_of_range_is_noop() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule(
                '-',
                Some('_'),
                ReplaceMode::Single {
                    occurrence: 5,
                    from_end: false,
                },
            )],
            validation_rules: vec![],
        };
        let result = config.validate_value(Some("a-b".to_string())).unwrap();
        assert_eq!(result, Some("a-b".to_string()));
    }

    #[test]
    fn test_string_replace_char_absent_char_is_noop() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule('-', None, ReplaceMode::All)],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("TXN202600123".to_string()))
            .unwrap();
        assert_eq!(result, Some("TXN202600123".to_string()));
    }

    #[test]
    fn test_string_replace_char_none_value_passthrough() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule('-', None, ReplaceMode::All)],
            validation_rules: vec![],
        };
        let result = config.validate_value(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_string_replace_char_multibyte_input() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule('-', None, ReplaceMode::All)],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("café-au-lait".to_string()))
            .unwrap();
        assert_eq!(result, Some("caféaulait".to_string()));

        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule(
                '-',
                Some('_'),
                ReplaceMode::Single {
                    occurrence: 0,
                    from_end: false,
                },
            )],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("café-au-lait".to_string()))
            .unwrap();
        assert_eq!(result, Some("café_au-lait".to_string()));
    }

    #[test]
    fn test_string_replace_char_after_trim() {
        let config = StringConfigV1 {
            transformation_rules: vec![trim_rule(), replace_char_rule('-', None, ReplaceMode::All)],
            validation_rules: vec![],
        };
        let result = config
            .validate_value(Some("  TXN-2026-00123  ".to_string()))
            .unwrap();
        assert_eq!(result, Some("TXN202600123".to_string()));
    }

    #[test]
    fn test_string_replace_char_then_length_validation() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule('-', None, ReplaceMode::All)],
            validation_rules: vec![StringValidationRule::V1(
                StringValidationRuleV1::MaxLength { value: 12 },
            )],
        };
        let result = config
            .validate_value(Some("TXN-2026-00123".to_string()))
            .unwrap();
        assert_eq!(result, Some("TXN202600123".to_string()));

        let result = config.validate_value(Some("TXN-2026-001234".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_string_replace_char_passes_config_validation() {
        let config = StringConfigV1 {
            transformation_rules: vec![replace_char_rule('-', None, ReplaceMode::All)],
            validation_rules: vec![],
        };
        assert!(config.validate_config().is_ok());
    }
}
