//! MinorUnit field validation tests

use common_types::{
    metadata_schema::{
        datatypes::minorunit::{
            MinorUnitConfig, MinorUnitConfigV1, MinorUnitValidationRule, MinorUnitValidationRuleV1,
        },
        FieldTypeConfig, FieldTypeConfigV1,
    },
    units::MinorUnit,
};
use rstest::rstest;

use super::{
    base_schema::BaseSchemaBuilder,
    helpers::{assert_field_error_matches, validate_single_field},
};
use crate::domain::metadata_schema::types::{FieldError, RuleError, ValueError};

#[rstest]
// Valid cases - no validation rules
#[case::valid_positive("1050", MinorUnitConfig::V1(MinorUnitConfigV1 { validation_rules: vec![], transformation_rules: vec![] }), Ok("1050"))]
#[case::valid_negative("-525", MinorUnitConfig::V1(MinorUnitConfigV1 { validation_rules: vec![], transformation_rules: vec![] }), Ok("-525"))]
#[case::valid_zero("0", MinorUnitConfig::V1(MinorUnitConfigV1 { validation_rules: vec![], transformation_rules: vec![] }), Ok("0"))]
#[case::valid_large("99999999", MinorUnitConfig::V1(MinorUnitConfigV1 { validation_rules: vec![], transformation_rules: vec![] }), Ok("99999999"))]
// MinValue validation - success
#[case::min_value_at_boundary(
    "1000",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MinValue {
            value: MinorUnit::new(1000)
        })],
        transformation_rules: vec![]
    }),
    Ok("1000")
)]
#[case::min_value_above(
    "1500",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MinValue {
            value: MinorUnit::new(1000)
        })],
        transformation_rules: vec![]
    }),
    Ok("1500")
)]
// MinValue validation - failure
#[case::min_value_below(
    "500",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MinValue {
            value: MinorUnit::new(1000)
        })],
        transformation_rules: vec![]
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("500".to_string()),
            rule_error: RuleError::ValueTooSmall {
                min_value: "1000".to_string(),
            },
        },
    })
)]
// MaxValue validation - success
#[case::max_value_at_boundary(
    "10000",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MaxValue {
            value: MinorUnit::new(10000)
        })],
        transformation_rules: vec![]
    }),
    Ok("10000")
)]
#[case::max_value_below(
    "5000",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MaxValue {
            value: MinorUnit::new(10000)
        })],
        transformation_rules: vec![]
    }),
    Ok("5000")
)]
// MaxValue validation - failure
#[case::max_value_above(
    "15000",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MaxValue {
            value: MinorUnit::new(10000)
        })],
        transformation_rules: vec![]
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("15000".to_string()),
            rule_error: RuleError::ValueTooLarge {
                max_value: "10000".to_string(),
            },
        },
    })
)]
// PositiveOnly validation - success
#[case::positive_only_positive(
    "100",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![]
    }),
    Ok("100")
)]
#[case::positive_only_zero(
    "0",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![]
    }),
    Ok("0")
)]
// PositiveOnly validation - failure
#[case::positive_only_negative(
    "-100",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![]
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("-100".to_string()),
            rule_error: RuleError::NegativeValue,
        },
    })
)]
// Combined validations - success
#[case::range_validation_within(
    "5000",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![
            MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MinValue {
                value: MinorUnit::new(1000)
            }),
            MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MaxValue {
                value: MinorUnit::new(10000)
            }),
        ],
        transformation_rules: vec![]
    }),
    Ok("5000")
)]
#[case::range_with_positive_only(
    "5000",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![
            MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly),
            MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MaxValue {
                value: MinorUnit::new(10000)
            }),
        ],
        transformation_rules: vec![]
    }),
    Ok("5000")
)]
// Combined validations - failure
#[case::range_validation_below_min(
    "500",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![
            MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MinValue {
                value: MinorUnit::new(1000)
            }),
            MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MaxValue {
                value: MinorUnit::new(10000)
            }),
        ],
        transformation_rules: vec![]
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("500".to_string()),
            rule_error: RuleError::ValueTooSmall {
                min_value: "1000".to_string(),
            },
        },
    })
)]
#[case::range_validation_above_max(
    "15000",
    MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![
            MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MinValue {
                value: MinorUnit::new(1000)
            }),
            MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::MaxValue {
                value: MinorUnit::new(10000)
            }),
        ],
        transformation_rules: vec![]
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("15000".to_string()),
            rule_error: RuleError::ValueTooLarge {
                max_value: "10000".to_string(),
            },
        },
    })
)]
// Invalid format cases
#[case::invalid_not_a_number(
    "abc",
    MinorUnitConfig::V1(MinorUnitConfigV1 { validation_rules: vec![], transformation_rules: vec![] }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidType {
            actual_value: "abc".to_string(),
        },
    })
)]
#[case::invalid_decimal(
    "10.50",
    MinorUnitConfig::V1(MinorUnitConfigV1 { validation_rules: vec![], transformation_rules: vec![] }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidType {
            actual_value: "10.50".to_string(),
        },
    })
)]
#[case::invalid_empty_string(
    "",
    MinorUnitConfig::V1(MinorUnitConfigV1 { validation_rules: vec![], transformation_rules: vec![] }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidType {
            actual_value: "".to_string(),
        },
    })
)]
fn test_minorunit_validations(
    #[case] input: &str,
    #[case] minorunit_config: MinorUnitConfig,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .add_metadata_field(
            "test_field",
            "test_field",
            FieldTypeConfig::V1(FieldTypeConfigV1::MinorUnit(minorunit_config)),
            true,
        )
        .build();

    let result = validate_single_field(&schema, "test_field", input);

    match (result, expected) {
        (Ok(actual), Ok(expected)) => {
            assert_eq!(actual, expected, "MinorUnit values don't match");
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
