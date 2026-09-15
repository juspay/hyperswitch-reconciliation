use common_types::metadata_schema::datatypes::enum_type::{
    EnumConfig, EnumConfigV1, EnumTransformationRule, EnumTransformationRuleV1,
};
use error_stack::{Report, ResultExt};
use std::collections::HashSet;

use crate::domain::metadata_schema::{
    config_validator::{validate_non_empty_string, ConfigValidator},
    data_validator::{ValueTransformer, ValueValidator},
    transformations,
    types::{ConfigFieldValidationError, RuleError, ValueError},
};

impl ValueValidator for EnumConfig {
    type DataType = String;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(enum_config_v1) => enum_config_v1.validate_value(value),
        }
    }
}

impl ValueValidator for EnumConfigV1 {
    type DataType = String;
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
            .map(|input_value| {
                self.mappings.get(&input_value).cloned().ok_or_else(|| {
                    Report::new(ValueError::InvalidEnumValue {
                        actual_value: input_value,
                        allowed_values: self.mappings.keys().cloned().collect(),
                    })
                })
            })
            .transpose()
    }
}

impl ValueTransformer for EnumTransformationRule {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(rule_v1) => rule_v1.transform_value(value),
        }
    }
}

impl ValueTransformer for EnumTransformationRuleV1 {
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

impl ConfigValidator for EnumConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
        }
    }
}

impl ConfigValidator for EnumConfigV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        if self.mappings.is_empty() {
            return Err(ConfigFieldValidationError::EmptyEnumMappings.into());
        }

        let mut seen_keys = HashSet::new();

        for (key, value) in &self.mappings {
            validate_non_empty_string(key)
                .change_context(ConfigFieldValidationError::EmptyEnumKey)?;

            if !seen_keys.insert(key.clone()) {
                return Err(
                    (ConfigFieldValidationError::DuplicateEnumKey { key: key.clone() }).into(),
                );
            }

            validate_non_empty_string(value)
                .change_context(ConfigFieldValidationError::EmptyEnumValue { key: key.clone() })?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use common_types::metadata_schema::datatypes::enum_type::{
        EnumConfigV1, EnumTransformationRule, EnumTransformationRuleV1,
    };

    use crate::domain::metadata_schema::data_validator::ValueValidator;

    fn trim_rule() -> EnumTransformationRule {
        EnumTransformationRule::V1(EnumTransformationRuleV1::Trim)
    }

    fn config(transformation_rules: Vec<EnumTransformationRule>) -> EnumConfigV1 {
        let mut mappings = HashMap::new();
        mappings.insert("settled".to_string(), "SETTLED".to_string());
        EnumConfigV1 {
            mappings,
            transformation_rules,
        }
    }

    #[test]
    fn test_enum_trim_before_lookup() {
        let cfg = config(vec![trim_rule()]);
        let result = cfg.validate_value(Some("  settled  ".to_string())).unwrap();
        assert_eq!(result, Some("SETTLED".to_string()));
    }

    #[test]
    fn test_enum_without_trim_fails_on_padded_input() {
        let cfg = config(vec![]);
        let result = cfg.validate_value(Some("  settled  ".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_none_value() {
        let cfg = config(vec![trim_rule()]);
        let result = cfg.validate_value(None).unwrap();
        assert_eq!(result, None);
    }
}
