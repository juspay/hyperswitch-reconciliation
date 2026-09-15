//! Enum field validation tests

use common_types::metadata_schema::{
    datatypes::enum_type::{EnumConfig, EnumConfigV1},
    FieldTypeConfig, FieldTypeConfigV1,
};
use rstest::rstest;
use std::collections::HashMap;

use super::{
    base_schema::BaseSchemaBuilder,
    helpers::{assert_field_error_matches, validate_single_field},
};
use crate::domain::metadata_schema::types::{FieldError, ValueError};

#[rstest]
// Valid mappings
#[case::direct_mapping(
    "pending",
    {
        let mut map = HashMap::new();
        map.insert("pending".to_string(), "PENDING".to_string());
        map.insert("approved".to_string(), "APPROVED".to_string());
        map.insert("rejected".to_string(), "REJECTED".to_string());
        EnumConfigV1 { mappings: map, transformation_rules: vec![] }
    },
    Ok("PENDING")
)]
#[case::different_output(
    "approved",
    {
        let mut map = HashMap::new();
        map.insert("pending".to_string(), "PENDING".to_string());
        map.insert("approved".to_string(), "APPROVED".to_string());
        map.insert("rejected".to_string(), "REJECTED".to_string());
        EnumConfigV1 { mappings: map, transformation_rules: vec![] }
    },
    Ok("APPROVED")
)]
#[case::numeric_mapping(
    "1",
    {
        let mut map = HashMap::new();
        map.insert("1".to_string(), "success".to_string());
        map.insert("0".to_string(), "failure".to_string());
        EnumConfigV1 { mappings: map, transformation_rules: vec![] }
    },
    Ok("success")
)]
#[case::case_sensitive_match(
    "Active",
    {
        let mut map = HashMap::new();
        map.insert("Active".to_string(), "ACTIVE".to_string());
        map.insert("active".to_string(), "active_lowercase".to_string());
        EnumConfigV1 { mappings: map, transformation_rules: vec![] }
    },
    Ok("ACTIVE")
)]
// Invalid - value not in mapping
#[case::value_not_found(
    "unknown",
    {
        let mut map = HashMap::new();
        map.insert("pending".to_string(), "PENDING".to_string());
        map.insert("approved".to_string(), "APPROVED".to_string());
        EnumConfigV1 { mappings: map, transformation_rules: vec![] }
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidEnumValue {
            actual_value: "unknown".to_string(),
            allowed_values: vec!["pending".to_string(), "approved".to_string()],
        },
    })
)]
#[case::case_mismatch(
    "PENDING",
    {
        let mut map = HashMap::new();
        map.insert("pending".to_string(), "PENDING".to_string());
        map.insert("approved".to_string(), "APPROVED".to_string());
        EnumConfigV1 { mappings: map, transformation_rules: vec![] }
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidEnumValue {
            actual_value: "PENDING".to_string(),
            allowed_values: vec!["pending".to_string(), "approved".to_string()],
        },
    })
)]
#[case::empty_string(
    "",
    {
        let mut map = HashMap::new();
        map.insert("pending".to_string(), "PENDING".to_string());
        EnumConfigV1 { mappings: map, transformation_rules: vec![] }
    },
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidEnumValue {
            actual_value: "".to_string(),
            allowed_values: vec!["pending".to_string()],
        },
    })
)]
fn test_enum_validations(
    #[case] input: &str,
    #[case] enum_config: EnumConfigV1,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .add_metadata_field(
            "test_field",
            "test_field",
            FieldTypeConfig::V1(FieldTypeConfigV1::Enum(EnumConfig::V1(enum_config))),
            true,
        )
        .build();

    let result = validate_single_field(&schema, "test_field", input);

    match (result, expected) {
        (Ok(actual), Ok(expected)) => {
            assert_eq!(actual, expected, "Enum values don't match");
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
