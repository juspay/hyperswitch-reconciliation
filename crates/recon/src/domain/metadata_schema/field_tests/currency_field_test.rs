//! Currency main field tests

use common_types::metadata_schema::fields::currency_field::{
    CurrencyFieldConfig, CurrencyFieldConfigV1,
};
use rstest::rstest;
use std::collections::HashMap;

use super::base_schema::BaseSchemaBuilder;
use crate::domain::metadata_schema::{
    types::{FieldError, MappedRow, RowValidationOutcome, ValueError},
    RowValidator,
};

#[rstest]
// Valid currencies
#[case::usd("USD", Ok("USD"))]
#[case::eur("EUR", Ok("EUR"))]
#[case::gbp("GBP", Ok("GBP"))]
#[case::jpy("JPY", Ok("JPY"))]
// Invalid currencies
#[case::invalid("XYZ", Err(FieldError::ValueError {
    identifier: "currency".to_string(),
    error: ValueError::InvalidCurrency {
        actual_value: Some("XYZ".to_string()),
        parse_error: "Unknown currency".to_string(),
    },
}))]
#[case::lowercase("usd", Err(FieldError::ValueError {
    identifier: "currency".to_string(),
    error: ValueError::InvalidCurrency {
        actual_value: Some("usd".to_string()),
        parse_error: "Matching variant not found".to_string(),
    },
}))]
#[case::empty("", Err(FieldError::ValueError {
    identifier: "currency".to_string(),
    error: ValueError::InvalidCurrency {
        actual_value: Some("".to_string()),
        parse_error: "Unknown currency".to_string(),
    },
}))]
fn test_currency_field_validations(
    #[case] input: &str,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .with_currency(CurrencyFieldConfig::V1(CurrencyFieldConfigV1 {
            identifier: "currency".to_string(),
            transformation_rules: vec![],
        }))
        .build();

    let mut fields = HashMap::new();
    fields.insert("amount".to_string(), Some("100".to_string()));
    fields.insert("currency".to_string(), Some(input.to_string()));
    fields.insert("balance_direction".to_string(), Some("credit".to_string()));
    fields.insert("effective_at".to_string(), Some("01-01-2024".to_string()));
    fields.insert("order_id".to_string(), Some("ORD-001".to_string()));

    let mapped_row = MappedRow::from_api_fields(fields, None);

    let result = schema.validate_row(&mapped_row);

    match expected {
        Ok(expected_currency) => {
            let RowValidationOutcome::Validated(entry) = &result else {
                panic!("Expected valid entry but validation failed: {result:?}");
            };
            let actual_currency = entry.row_data.currency.to_string();
            assert_eq!(
                actual_currency, expected_currency,
                "Currency values don't match"
            );
        }
        Err(expected_err) => {
            let RowValidationOutcome::Rejected(rejection) = &result else {
                panic!("Expected validation error but got success");
            };
            let currency_err = rejection
                .field_errors
                .iter()
                .map(|located| &located.value)
                .find(|e| e.get_identifier() == "currency")
                .expect("Should have error for currency field");

            use super::helpers::assert_field_error_matches;
            assert_field_error_matches(currency_err, &expected_err);
        }
    }
}
