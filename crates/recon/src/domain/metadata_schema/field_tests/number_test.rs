//! Number field validation tests

use common_types::metadata_schema::{
    datatypes::number::{
        NumberConfig, NumberConfigV1, NumberValidationRule, NumberValidationRuleV1,
    },
    FieldTypeConfig, FieldTypeConfigV1,
};
use rstest::rstest;

use super::{
    base_schema::BaseSchemaBuilder,
    helpers::{assert_field_error_matches, validate_single_field},
};
use crate::domain::metadata_schema::types::{FieldError, RuleError, ValueError};

#[rstest]
// Valid cases - no validation rules
#[case::valid_positive("10.5", NumberConfig::V1(NumberConfigV1 { validation_rules: vec![], transformation_rules: vec![] }), Ok("10.5"))]
#[case::valid_negative("-5.25", NumberConfig::V1(NumberConfigV1 { validation_rules: vec![], transformation_rules: vec![] }), Ok("-5.25"))]
#[case::valid_zero("0", NumberConfig::V1(NumberConfigV1 { validation_rules: vec![], transformation_rules: vec![] }), Ok("0"))]
#[case::valid_large("999999.99", NumberConfig::V1(NumberConfigV1 { validation_rules: vec![], transformation_rules: vec![] }), Ok("999999.99"))]
// MinValue validation - success
#[case::min_value_at_boundary(
    "10.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![NumberValidationRule::V1(NumberValidationRuleV1::MinValue { value: 10.0 })],
        transformation_rules: vec![],
    }),
    Ok("10.0")
)]
#[case::min_value_above(
    "15.5",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![NumberValidationRule::V1(NumberValidationRuleV1::MinValue { value: 10.0 })],
        transformation_rules: vec![],
    }),
    Ok("15.5")
)]
// MinValue validation - failure
#[case::min_value_below(
    "5.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![NumberValidationRule::V1(NumberValidationRuleV1::MinValue { value: 10.0 })],
        transformation_rules: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("5".to_string()),
            rule_error: RuleError::ValueTooSmall {
                min_value: "10".to_string(),
            },
        },
    })
)]
// MaxValue validation - success
#[case::max_value_at_boundary(
    "100.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![NumberValidationRule::V1(NumberValidationRuleV1::MaxValue { value: 100.0 })],
        transformation_rules: vec![],
    }),
    Ok("100.0")
)]
#[case::max_value_below(
    "50.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![NumberValidationRule::V1(NumberValidationRuleV1::MaxValue { value: 100.0 })],
        transformation_rules: vec![],
    }),
    Ok("50.0")
)]
// MaxValue validation - failure
#[case::max_value_above(
    "150.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![NumberValidationRule::V1(NumberValidationRuleV1::MaxValue { value: 100.0 })],
        transformation_rules: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("150".to_string()),
            rule_error: RuleError::ValueTooLarge {
                max_value: "100".to_string(),
            },
        },
    })
)]
// Both MinValue and MaxValue - success
#[case::range_validation_within(
    "50.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![
            NumberValidationRule::V1(NumberValidationRuleV1::MinValue { value: 10.0 }),
            NumberValidationRule::V1(NumberValidationRuleV1::MaxValue { value: 100.0 }),
        ],
        transformation_rules: vec![],
    }),
    Ok("50.0")
)]
#[case::range_validation_at_min(
    "10.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![
            NumberValidationRule::V1(NumberValidationRuleV1::MinValue { value: 10.0 }),
            NumberValidationRule::V1(NumberValidationRuleV1::MaxValue { value: 100.0 }),
        ],
        transformation_rules: vec![],
    }),
    Ok("10.0")
)]
#[case::range_validation_at_max(
    "100.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![
            NumberValidationRule::V1(NumberValidationRuleV1::MinValue { value: 10.0 }),
            NumberValidationRule::V1(NumberValidationRuleV1::MaxValue { value: 100.0 }),
        ],
        transformation_rules: vec![],
    }),
    Ok("100.0")
)]
// Both MinValue and MaxValue - failure
#[case::range_validation_below_min(
    "5.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![
            NumberValidationRule::V1(NumberValidationRuleV1::MinValue { value: 10.0 }),
            NumberValidationRule::V1(NumberValidationRuleV1::MaxValue { value: 100.0 }),
        ],
        transformation_rules: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("5".to_string()),
            rule_error: RuleError::ValueTooSmall {
                min_value: "10".to_string(),
            },
        },
    })
)]
#[case::range_validation_above_max(
    "150.0",
    NumberConfig::V1(NumberConfigV1 {
        validation_rules: vec![
            NumberValidationRule::V1(NumberValidationRuleV1::MinValue { value: 10.0 }),
            NumberValidationRule::V1(NumberValidationRuleV1::MaxValue { value: 100.0 }),
        ],
        transformation_rules: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("150".to_string()),
            rule_error: RuleError::ValueTooLarge {
                max_value: "100".to_string(),
            },
        },
    })
)]
// Invalid format cases
#[case::invalid_not_a_number(
    "abc",
    NumberConfig::V1(NumberConfigV1 { validation_rules: vec![], transformation_rules: vec![] }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidType {
            actual_value: "abc".to_string(),
        },
    })
)]
#[case::invalid_empty_string(
    "",
    NumberConfig::V1(NumberConfigV1 { validation_rules: vec![], transformation_rules: vec![] }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidType {
            actual_value: "".to_string(),
        },
    })
)]
fn test_number_validations(
    #[case] input: &str,
    #[case] number_config: NumberConfig,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .add_metadata_field(
            "test_field",
            "test_field",
            FieldTypeConfig::V1(FieldTypeConfigV1::Number(number_config)),
            true,
        )
        .build();

    let result = validate_single_field(&schema, "test_field", input);

    match (result, expected) {
        (Ok(actual), Ok(expected)) => {
            // For numbers, we need to compare the parsed values since formatting might differ
            let actual_num: f64 = actual.parse().expect("Should be valid number");
            let expected_num: f64 = expected.parse().expect("Should be valid number");
            assert!(
                (actual_num - expected_num).abs() < f64::EPSILON,
                "Numbers don't match: {} vs {}",
                actual_num,
                expected_num
            );
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
