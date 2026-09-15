//! Order ID main field tests

use common_types::metadata_schema::fields::order_id_field::{
    OrderIdFieldConfig, OrderIdFieldConfigV1,
};
use rstest::rstest;
use std::collections::HashMap;

use super::base_schema::BaseSchemaBuilder;
use crate::domain::metadata_schema::{
    types::{FieldError, MappedRow, RowValidationOutcome, RuleError, ValueError},
    RowValidator,
};

#[rstest]
// Valid order IDs
#[case::simple("ORD-001".to_string(), Ok("ORD-001".to_string()))]
#[case::uuid(
    "550e8400-e29b-41d4-a716-446655440000".to_string(),
    Ok("550e8400-e29b-41d4-a716-446655440000".to_string())
)]
#[case::alphanumeric("ABC123XYZ".to_string(), Ok("ABC123XYZ".to_string()))]
#[case::with_special_chars("ORD_2024_001".to_string(), Ok("ORD_2024_001".to_string()))]
#[case::max_length("A".repeat(64), Ok("A".repeat(64)))]
// Invalid - exceeds max length (64 characters enforced by default)
#[case::too_long(
    "A".repeat(65),
    Err(FieldError::ValueError {
        identifier: "order_id".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("A".repeat(65)),
            rule_error: RuleError::ValueTooLong {
                max_length: "64".to_string(),
            },
        },
    })
)]
// Edge cases
#[case::empty("".to_string(), Ok("".to_string()))]
#[case::single_char("A".to_string(), Ok("A".to_string()))]
fn test_order_id_field_validations(
    #[case] input: String,
    #[case] expected: Result<String, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .with_order_id(OrderIdFieldConfig::V1(OrderIdFieldConfigV1 {
            identifier: "order_id".to_string(),
            transformation_rules: vec![], // Need to add test cases for this as well
        }))
        .build();

    let mut fields = HashMap::new();
    fields.insert("amount".to_string(), Some("100".to_string()));
    fields.insert("currency".to_string(), Some("USD".to_string()));
    fields.insert("balance_direction".to_string(), Some("credit".to_string()));
    fields.insert("effective_at".to_string(), Some("01-01-2024".to_string()));
    fields.insert("order_id".to_string(), Some(input.to_string()));

    let mapped_row = MappedRow::from_api_fields(fields, None);

    let result = schema.validate_row(&mapped_row);

    match expected {
        Ok(expected_order_id) => {
            let RowValidationOutcome::Validated(entry) = &result else {
                panic!("Expected valid entry but validation failed: {result:?}");
            };
            let actual_order_id = entry.row_data.order_id.clone();
            assert_eq!(
                actual_order_id, expected_order_id,
                "Order ID values don't match"
            );
        }
        Err(expected_err) => {
            let RowValidationOutcome::Rejected(rejection) = &result else {
                panic!("Expected validation error but got success");
            };
            let order_id_err = rejection
                .field_errors
                .iter()
                .map(|located| &located.value)
                .find(|e| e.get_identifier() == "order_id")
                .expect("Should have error for order_id field");

            match (&expected_err, order_id_err) {
                (
                    FieldError::ValueError {
                        error: expected_value_err,
                        ..
                    },
                    FieldError::ValueError {
                        error: actual_value_err,
                        ..
                    },
                ) => {
                    use super::helpers::assert_value_error_matches;
                    assert_value_error_matches(actual_value_err, expected_value_err);
                }
                _ => panic!(
                    "Error type mismatch: expected {:?}, got {:?}",
                    expected_err, order_id_err
                ),
            }
        }
    }
}
