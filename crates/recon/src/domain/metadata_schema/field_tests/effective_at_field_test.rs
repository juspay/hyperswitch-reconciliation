//! Effective at main field tests

use common_types::metadata_schema::{
    datatypes::datetime::{DateFormatConfig, DateOrder, DateTimeFormat, Delimiter},
    fields::effective_at_field::{EffectiveAtFieldConfig, EffectiveAtFieldConfigV2},
};
use rstest::rstest;
use std::collections::HashMap;

use super::base_schema::BaseSchemaBuilder;
use crate::domain::metadata_schema::{
    types::{FieldError, MappedRow, RowValidationOutcome, ValueError},
    RowValidator,
};

#[rstest]
// Valid dates
#[case::valid_date("15-03-2024", Ok("2024-03-15 0:00:00.0"))]
#[case::single_digit("5-3-2024", Ok("2024-03-05 0:00:00.0"))]
#[case::leap_year("29-02-2024", Ok("2024-02-29 0:00:00.0"))]
// Invalid dates
#[case::invalid_format("15/03/2024", Err(FieldError::ValueError {
    identifier: "effective_at".to_string(),
    error: ValueError::InvalidFormat {
        actual_value: Some("15/03/2024".to_string()),
    },
}))]
#[case::invalid_date("32-13-2024", Err(FieldError::ValueError {
    identifier: "effective_at".to_string(),
    error: ValueError::InvalidFormat {
        actual_value: Some("32-13-2024".to_string()),
    },
}))]
#[case::empty("", Err(FieldError::ValueError {
    identifier: "effective_at".to_string(),
    error: ValueError::InvalidFormat {
        actual_value: Some("".to_string()),
    },
}))]
fn test_effective_at_field_validations(
    #[case] input: &str,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .with_effective_at(EffectiveAtFieldConfig::V2(EffectiveAtFieldConfigV2 {
            identifier: "effective_at".to_string(),
            date_time_format: DateTimeFormat {
                date_format: DateFormatConfig {
                    order: DateOrder::DayMonthYear,
                    delimiter: Delimiter::Hyphen,
                },
                time_format: None,
            },
            transformation_rules: vec![],
            post_parse_transformations: vec![],
        }))
        .build();

    let mut fields = HashMap::new();
    fields.insert("amount".to_string(), Some("100".to_string()));
    fields.insert("currency".to_string(), Some("USD".to_string()));
    fields.insert("balance_direction".to_string(), Some("credit".to_string()));
    fields.insert("effective_at".to_string(), Some(input.to_string()));
    fields.insert("order_id".to_string(), Some("ORD-001".to_string()));

    let mapped_row = MappedRow::from_api_fields(fields, None);

    let result = schema.validate_row(&mapped_row);

    match expected {
        Ok(expected_date) => {
            let RowValidationOutcome::Validated(entry) = &result else {
                panic!("Expected valid entry but validation failed: {result:?}");
            };
            let actual_date = entry.row_data.effective_at.to_string();
            assert_eq!(
                actual_date, expected_date,
                "Effective at values don't match"
            );
        }
        Err(expected_err) => {
            let RowValidationOutcome::Rejected(rejection) = &result else {
                panic!("Expected validation error but got success");
            };
            let effective_at_err = rejection
                .field_errors
                .iter()
                .map(|located| &located.value)
                .find(|e| e.get_identifier() == "effective_at")
                .expect("Should have error for effective_at field");

            use super::helpers::assert_field_error_matches;
            assert_field_error_matches(effective_at_err, &expected_err);
        }
    }
}
