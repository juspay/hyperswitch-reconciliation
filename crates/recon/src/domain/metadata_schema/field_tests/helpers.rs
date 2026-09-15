//! Test helper functions for field validation

use std::collections::HashMap;

use crate::domain::metadata_schema::{
    types::{FieldError, MappedRow, RowValidationOutcome, RuleError, ValueError},
    RowValidator,
};
use common_types::metadata_schema::MetadataSchemaData;

/// Validate a single field value against a schema
/// Returns the transformed/validated value on success, or the error on failure
pub fn validate_single_field(
    schema: &MetadataSchemaData,
    field_identifier: &str,
    value: &str,
) -> Result<String, FieldError> {
    // Create minimal mapped row with required main fields + test field
    let mut fields = HashMap::new();
    fields.insert("amount".to_string(), Some("100".to_string()));
    fields.insert("currency".to_string(), Some("USD".to_string()));
    fields.insert("balance_direction".to_string(), Some("credit".to_string()));
    fields.insert("effective_at".to_string(), Some("01-01-2024".to_string()));
    fields.insert("order_id".to_string(), Some("ORD-001".to_string()));
    fields.insert(field_identifier.to_string(), Some(value.to_string()));

    let mapped_row = MappedRow::from_api_fields(fields, None);

    let result = schema.validate_row(&mapped_row);

    // Check if validation succeeded
    if let RowValidationOutcome::Validated(entry) = &result {
        if let Some(field_value) = entry.row_data.metadata.get(field_identifier) {
            // Handle different JSON value types
            let string_value = match field_value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".to_string(),
                serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                    field_value.to_string()
                }
            };
            return Ok(string_value);
        }

        let available_keys = entry
            .row_data
            .metadata
            .as_object()
            .map(|obj| obj.keys().collect::<Vec<_>>())
            .unwrap_or_default();

        panic!(
            "Field {} not found in validated metadata. Available keys: {:?}",
            field_identifier, available_keys
        );
    }

    // Validation failed - extract the error
    if let RowValidationOutcome::Rejected(rejection) = &result {
        // Find the error for our field
        for located in &rejection.field_errors {
            match &located.value {
                FieldError::ValueError { identifier, error } if identifier == field_identifier => {
                    return Err(FieldError::ValueError {
                        identifier: identifier.clone(),
                        error: error.clone(),
                    });
                }
                FieldError::RequiredFieldMissing(id) if id == field_identifier => {
                    return Err(FieldError::RequiredFieldMissing(id.clone()));
                }
                FieldError::FieldNotMapped(id) if id == field_identifier => {
                    return Err(FieldError::FieldNotMapped(id.clone()));
                }
                FieldError::RequiredFieldMissing(_)
                | FieldError::FieldNotMapped(_)
                | FieldError::ValueError { .. } => {
                    continue;
                }
            }
        }
    }

    panic!("Unexpected validation result");
}

/// Check if a RuleError matches the expected type
pub fn assert_rule_error_matches(actual: &RuleError, expected: &RuleError) {
    match (actual, expected) {
        (RuleError::ValueTooShort { .. }, RuleError::ValueTooShort { .. }) => {}
        (RuleError::ValueTooLong { .. }, RuleError::ValueTooLong { .. }) => {}
        (RuleError::ValueTooSmall { .. }, RuleError::ValueTooSmall { .. }) => {}
        (RuleError::ValueTooLarge { .. }, RuleError::ValueTooLarge { .. }) => {}
        (RuleError::NegativeValue, RuleError::NegativeValue) => {}
        (RuleError::PrefixNotFound { .. }, RuleError::PrefixNotFound { .. }) => {}
        (RuleError::SuffixNotFound { .. }, RuleError::SuffixNotFound { .. }) => {}
        (RuleError::JsonParseFailed, RuleError::JsonParseFailed) => {}
        (RuleError::JsonKeyNotFound { .. }, RuleError::JsonKeyNotFound { .. }) => {}
        (RuleError::UnmatchedRegex { .. }, RuleError::UnmatchedRegex { .. }) => {}
        _ => panic!(
            "RuleError type mismatch: actual={:?}, expected={:?}",
            actual, expected
        ),
    }
}

/// Check if a ValueError matches the expected type
pub fn assert_value_error_matches(actual: &ValueError, expected: &ValueError) {
    match (actual, expected) {
        (ValueError::InvalidType { .. }, ValueError::InvalidType { .. }) => {}
        (ValueError::InvalidFormat { .. }, ValueError::InvalidFormat { .. }) => {}
        (
            ValueError::InvalidBalanceDirection { .. },
            ValueError::InvalidBalanceDirection { .. },
        ) => {}
        (ValueError::InvalidCurrency { .. }, ValueError::InvalidCurrency { .. }) => {}
        (ValueError::InvalidEnumValue { .. }, ValueError::InvalidEnumValue { .. }) => {}
        (
            ValueError::RuleError {
                rule_error: actual_rule,
                ..
            },
            ValueError::RuleError {
                rule_error: expected_rule,
                ..
            },
        ) => {
            assert_rule_error_matches(actual_rule, expected_rule);
        }
        _ => panic!(
            "ValueError type mismatch: actual={:?}, expected={:?}",
            actual, expected
        ),
    }
}

/// Check if a FieldError matches the expected type
pub fn assert_field_error_matches(actual: &FieldError, expected: &FieldError) {
    match (actual, expected) {
        (
            FieldError::RequiredFieldMissing(actual_field),
            FieldError::RequiredFieldMissing(expected_field),
        ) => {
            assert_eq!(actual_field, expected_field);
        }
        (
            FieldError::ValueError {
                error: actual_err, ..
            },
            FieldError::ValueError {
                error: expected_err,
                ..
            },
        ) => {
            assert_value_error_matches(actual_err, expected_err);
        }
        _ => panic!(
            "FieldError type mismatch: actual={:?}, expected={:?}",
            actual, expected
        ),
    }
}
