//! Amount main field tests

use common_types::metadata_schema::{
    datatypes::{
        majorunit::{
            AmountDelimiter, MajorUnitConfig, MajorUnitConfigV1, MajorUnitValidationRule,
            MajorUnitValidationRuleV1,
        },
        minorunit::{
            MinorUnitConfig, MinorUnitConfigV1, MinorUnitValidationRule, MinorUnitValidationRuleV1,
        },
    },
    fields::amount_field::{AmountFieldConfig, AmountFieldConfigV2, AmountUnitConfig},
};
use rstest::rstest;
use std::collections::HashMap;

use super::base_schema::BaseSchemaBuilder;
use crate::domain::metadata_schema::{
    types::{FieldError, MappedRow, RowValidationOutcome, RuleError, ValueError},
    RowValidator,
};

#[rstest]
// Valid amounts
#[case::positive_amount(
    "1000",
    AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![],
    })),
    Ok(1000)
)]
#[case::zero(
    "0",
    AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![],
    })),
    Ok(0)
)]
#[case::large_amount(
    "999999999",
    AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![],
    })),
    Ok(999999999)
)]
// Invalid - negative values (PositiveOnly rule is enforced)
#[case::negative_amount(
    "-100",
    AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![],
    })),
    Err(FieldError::ValueError {
        identifier: "amount".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("-100".to_string()),
            rule_error: RuleError::NegativeValue,
        },
    })
)]
// Invalid formats
#[case::decimal_for_minorunit_config(
    "10.50",
    AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![],
    })),
    Err(FieldError::ValueError {
        identifier: "amount".to_string(),
        error: ValueError::InvalidType {
            actual_value: "10.50".to_string(),
        },
    })
)]
#[case::decimal_for_majorunit_config(
    "10.50",
    AmountUnitConfig::MajorUnit(MajorUnitConfig::V1(MajorUnitConfigV1 {
        validation_rules: vec![MajorUnitValidationRule::V1(MajorUnitValidationRuleV1::PositiveOnly)],
        delimiter: AmountDelimiter::Dot,
        transformation_rules: vec![],
    })),
    Ok(1050)
)]
#[case::text(
    "abc",
    AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![],
    })),
    Err(FieldError::ValueError {
        identifier: "amount".to_string(),
        error: ValueError::InvalidType {
            actual_value: "abc".to_string(),
        },
    })
)]
#[case::empty(
    "",
    AmountUnitConfig::MinorUnit(MinorUnitConfig::V1(MinorUnitConfigV1 {
        validation_rules: vec![MinorUnitValidationRule::V1(MinorUnitValidationRuleV1::PositiveOnly)],
        transformation_rules: vec![],
    })),
    Err(FieldError::ValueError {
        identifier: "amount".to_string(),
        error: ValueError::InvalidType {
            actual_value: "".to_string(),
        },
    })
)]
fn test_amount_field_validations(
    #[case] input: &str,
    #[case] unit_config: AmountUnitConfig,
    #[case] expected: Result<i64, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .with_amount(AmountFieldConfig::V2(AmountFieldConfigV2 {
            identifier: "amount".to_string(),
            unit_config,
        }))
        .build();

    // Create test row
    let mut fields = HashMap::new();
    fields.insert("amount".to_string(), Some(input.to_string()));
    fields.insert("currency".to_string(), Some("USD".to_string()));
    fields.insert("balance_direction".to_string(), Some("credit".to_string()));
    fields.insert("effective_at".to_string(), Some("01-01-2024".to_string()));
    fields.insert("order_id".to_string(), Some("ORD-001".to_string()));

    let mapped_row = MappedRow::from_api_fields(fields, None);

    let result = schema.validate_row(&mapped_row);

    match expected {
        Ok(expected_amount) => {
            let RowValidationOutcome::Validated(entry) = &result else {
                panic!("Expected valid entry but validation failed: {result:?}");
            };
            let actual_amount = entry.row_data.amount.get_amount_as_i64();
            assert_eq!(actual_amount, expected_amount, "Amount values don't match");
        }
        Err(expected_err) => {
            let RowValidationOutcome::Rejected(rejection) = &result else {
                panic!("Expected validation error but got success");
            };
            let amount_err = rejection
                .field_errors
                .iter()
                .map(|located| &located.value)
                .find(|e| e.get_identifier() == "amount")
                .expect("Should have error for amount field");

            match (&expected_err, amount_err) {
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
                    expected_err, amount_err
                ),
            }
        }
    }
}
