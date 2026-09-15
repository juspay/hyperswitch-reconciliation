//! BalanceDirection field validation tests

use common_types::metadata_schema::{
    datatypes::balance_direction::{BalanceDirectionConfig, BalanceDirectionConfigV1},
    FieldTypeConfig, FieldTypeConfigV1,
};
use rstest::rstest;

use super::{
    base_schema::BaseSchemaBuilder,
    helpers::{assert_field_error_matches, validate_single_field},
};
use crate::domain::metadata_schema::types::{FieldError, ValueError};

#[rstest]
// Valid credit values
#[case::credit_standard(
    "credit",
    BalanceDirectionConfigV1 {
        credit_values: vec!["credit".to_string()],
        debit_values: vec!["debit".to_string()],
        transformation_rules: vec![],
    },
    Ok("credit")
)]
#[case::credit_uppercase(
    "CREDIT",
    BalanceDirectionConfigV1 {
        credit_values: vec!["CREDIT".to_string()],
        debit_values: vec!["DEBIT".to_string()],
        transformation_rules: vec![],
    },
    Ok("credit")
)]
#[case::credit_plus(
    "+",
    BalanceDirectionConfigV1 {
        credit_values: vec!["+".to_string()],
        debit_values: vec!["-".to_string()],
        transformation_rules: vec![],
    },
    Ok("credit")
)]
#[case::credit_custom(
    "IN",
    BalanceDirectionConfigV1 {
        credit_values: vec!["IN".to_string(), "INCOMING".to_string()],
        debit_values: vec!["OUT".to_string(), "OUTGOING".to_string()],
        transformation_rules: vec![],
    },
    Ok("credit")
)]
// Valid debit values
#[case::debit_standard(
    "debit",
    BalanceDirectionConfigV1 {
        credit_values: vec!["credit".to_string()],
        debit_values: vec!["debit".to_string()],
        transformation_rules: vec![],
    },
    Ok("debit")
)]
#[case::debit_uppercase(
    "DEBIT",
    BalanceDirectionConfigV1 {
        credit_values: vec!["CREDIT".to_string()],
        debit_values: vec!["DEBIT".to_string()],
        transformation_rules: vec![],
    },
    Ok("debit")
)]
#[case::debit_minus(
    "-",
    BalanceDirectionConfigV1 {
        credit_values: vec!["+".to_string()],
        debit_values: vec!["-".to_string()],
        transformation_rules: vec![],
    },
    Ok("debit")
)]
#[case::debit_custom(
    "OUT",
    BalanceDirectionConfigV1 {
        credit_values: vec!["IN".to_string(), "INCOMING".to_string()],
        debit_values: vec!["OUT".to_string(), "OUTGOING".to_string()],
        transformation_rules: vec![],
    },
    Ok("debit")
)]
// Multiple credit values - any match should work
#[case::credit_multiple_values_first(
    "IN",
    BalanceDirectionConfigV1 {
        credit_values: vec!["IN".to_string(), "INCOMING".to_string(), "CREDIT".to_string()],
        debit_values: vec!["OUT".to_string()],
        transformation_rules: vec![],
    },
    Ok("credit")
)]
#[case::credit_multiple_values_second(
    "INCOMING",
    BalanceDirectionConfigV1 {
        credit_values: vec!["IN".to_string(), "INCOMING".to_string(), "CREDIT".to_string()],
        debit_values: vec!["OUT".to_string()],
        transformation_rules: vec![],
    },
    Ok("credit")
)]
// Invalid balance direction
#[case::invalid_value(
    "invalid",
    BalanceDirectionConfigV1 {
        credit_values: vec!["credit".to_string()],
        debit_values: vec!["debit".to_string()],
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidBalanceDirection {
            actual_value: "invalid".to_string(),
        },
    })
)]
#[case::empty_string(
    "",
    BalanceDirectionConfigV1 {
        credit_values: vec!["credit".to_string()],
        debit_values: vec!["debit".to_string()],
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidBalanceDirection {
            actual_value: "".to_string(),
        },
    })
)]
#[case::wrong_case_when_not_configured(
    "Credit",
    BalanceDirectionConfigV1 {
        credit_values: vec!["credit".to_string()],
        debit_values: vec!["debit".to_string()],
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidBalanceDirection {
            actual_value: "Credit".to_string(),
        },
    })
)]
fn test_balance_direction_validations(
    #[case] input: &str,
    #[case] balance_direction_config: BalanceDirectionConfigV1,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .add_metadata_field(
            "test_field",
            "test_field",
            FieldTypeConfig::V1(FieldTypeConfigV1::BalanceDirection(
                BalanceDirectionConfig::V1(balance_direction_config),
            )),
            true,
        )
        .build();

    let result = validate_single_field(&schema, "test_field", input);

    match (result, expected) {
        (Ok(actual), Ok(expected)) => {
            assert_eq!(actual, expected, "BalanceDirection values don't match");
        }
        (Err(actual_err), Err(expected_err)) => {
            assert_field_error_matches(&actual_err, &expected_err);
        }
        (Ok(actual), Err(expected)) => {
            panic!("Expected error {:?}, but got success: {}", expected, actual);
        }
        (Err(actual), Ok(expected)) => {
            panic!("Expected success {}, but got error: {:?}", expected, actual);
        }
    }
}
