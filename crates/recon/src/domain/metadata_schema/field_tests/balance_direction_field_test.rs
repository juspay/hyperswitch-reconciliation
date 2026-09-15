//! Balance direction main field tests

use common_types::metadata_schema::fields::balance_direction_field::{
    BalanceDirectionFieldConfig, BalanceDirectionFieldConfigV1,
};
use rstest::rstest;
use std::collections::HashMap;

use super::base_schema::BaseSchemaBuilder;
use crate::domain::metadata_schema::{
    types::{FieldError, MappedRow, RowValidationOutcome, ValueError},
    RowValidator,
};

#[rstest]
// Valid credit values
#[case::credit("credit", Ok("credit"))]
#[case::custom_credit("IN", Ok("credit"))]
// Valid debit values
#[case::debit("debit", Ok("debit"))]
#[case::custom_debit("OUT", Ok("debit"))]
// Invalid values
#[case::invalid("invalid", Err(FieldError::ValueError {
    identifier: "balance_direction".to_string(),
    error: ValueError::InvalidBalanceDirection {
        actual_value: "invalid".to_string(),
    },
}))]
#[case::empty("", Err(FieldError::ValueError {
    identifier: "balance_direction".to_string(),
    error: ValueError::InvalidBalanceDirection {
        actual_value: "".to_string(),
    },
}))]
fn test_balance_direction_field_validations(
    #[case] input: &str,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .with_balance_direction(BalanceDirectionFieldConfig::V1(
            BalanceDirectionFieldConfigV1 {
                identifier: "balance_direction".to_string(),
                credit_values: vec!["credit".to_string(), "IN".to_string()],
                debit_values: vec!["debit".to_string(), "OUT".to_string()],
                transformation_rules: vec![],
            },
        ))
        .build();

    let mut fields = HashMap::new();
    fields.insert("amount".to_string(), Some("100".to_string()));
    fields.insert("currency".to_string(), Some("USD".to_string()));
    fields.insert("balance_direction".to_string(), Some(input.to_string()));
    fields.insert("effective_at".to_string(), Some("01-01-2024".to_string()));
    fields.insert("order_id".to_string(), Some("ORD-001".to_string()));

    let mapped_row = MappedRow::from_api_fields(fields, None);

    let result = schema.validate_row(&mapped_row);

    match expected {
        Ok(expected_direction) => {
            let RowValidationOutcome::Validated(entry) = &result else {
                panic!("Expected valid entry but validation failed: {result:?}");
            };
            let actual_direction = entry.row_data.balance_direction.to_string();
            assert_eq!(
                actual_direction, expected_direction,
                "Balance direction values don't match"
            );
        }
        Err(expected_err) => {
            let RowValidationOutcome::Rejected(rejection) = &result else {
                panic!("Expected validation error but got success");
            };
            let balance_err = rejection
                .field_errors
                .iter()
                .map(|located| &located.value)
                .find(|e| e.get_identifier() == "balance_direction")
                .expect("Should have error for balance_direction field");

            use super::helpers::assert_field_error_matches;
            assert_field_error_matches(balance_err, &expected_err);
        }
    }
}
