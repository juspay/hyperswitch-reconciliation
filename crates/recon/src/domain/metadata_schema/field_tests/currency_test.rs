//! Currency field validation tests

use common_types::metadata_schema::{
    datatypes::currency::{CurrencyConfig, CurrencyConfigV1},
    FieldTypeConfig, FieldTypeConfigV1,
};
use rstest::rstest;

use super::{
    base_schema::BaseSchemaBuilder,
    helpers::{assert_field_error_matches, validate_single_field},
};
use crate::domain::metadata_schema::types::{FieldError, ValueError};

#[rstest]
// Valid currency codes
#[case::usd("USD", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("USD"))]
#[case::eur("EUR", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("EUR"))]
#[case::gbp("GBP", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("GBP"))]
#[case::jpy("JPY", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("JPY"))]
#[case::inr("INR", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("INR"))]
#[case::aud("AUD", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("AUD"))]
#[case::cad("CAD", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("CAD"))]
#[case::chf("CHF", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("CHF"))]
#[case::cny("CNY", CurrencyConfigV1 {transformation_rules: vec![]}, Ok("CNY"))]
#[case::lowercase_usd(
    "usd",
    CurrencyConfigV1 {
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidCurrency {
            actual_value: Some("usd".to_string()),
            parse_error: "Matching variant not found".to_string(),
        },
    })
)]
// Invalid currency codes
#[case::invalid_code(
    "XYZ",
    CurrencyConfigV1 {
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidCurrency {
            actual_value: Some("XYZ".to_string()),
            parse_error: "Unknown currency".to_string(),
        },
    })
)]
#[case::empty_string(
    "",
    CurrencyConfigV1 {
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidCurrency {
            actual_value: Some("".to_string()),
            parse_error: "Unknown currency".to_string(),
        },
    })
)]
#[case::numbers(
    "123",
    CurrencyConfigV1 {
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidCurrency {
            actual_value: Some("123".to_string()),
            parse_error: "Unknown currency".to_string(),
        },
    })
)]
#[case::too_long(
    "USDD",
    CurrencyConfigV1 {
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidCurrency {
            actual_value: Some("USDD".to_string()),
            parse_error: "Unknown currency".to_string(),
        },
    })
)]
#[case::too_short(
    "US",
    CurrencyConfigV1 {
        transformation_rules: vec![],
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidCurrency {
            actual_value: Some("US".to_string()),
            parse_error: "Unknown currency".to_string(),
        },
    })
)]
fn test_currency_validations(
    #[case] input: &str,
    #[case] _currency_config: CurrencyConfigV1,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .add_metadata_field(
            "test_field",
            "test_field",
            FieldTypeConfig::V1(FieldTypeConfigV1::Currency(CurrencyConfig::V1(
                CurrencyConfigV1 {
                    transformation_rules: vec![],
                },
            ))),
            true,
        )
        .build();

    let result = validate_single_field(&schema, "test_field", input);

    match (result, expected) {
        (Ok(actual), Ok(expected)) => {
            assert_eq!(actual, expected, "Currency values don't match");
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
