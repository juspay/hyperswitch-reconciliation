//! Comprehensive string field tests
//!
//! Tests all string transformations, validations, and their combinations

use super::{base_schema::BaseSchemaBuilder, helpers::*};
use crate::domain::metadata_schema::types::{FieldError, RuleError, ValueError};
use common_types::metadata_schema::{
    datatypes::{
        majorunit::ReplaceMode,
        string::{
            StringConfig, StringConfigV1, StringTransformationRule, StringTransformationRuleV1,
            StringValidationRule, StringValidationRuleV1,
        },
    },
    FieldTypeConfig, FieldTypeConfigV1,
};
use rstest::rstest;

#[rstest]
// ============================================================================
// ToUpperCase Transformation Tests
// ============================================================================
#[case::to_uppercase_simple_lowercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "hello",
    Ok("HELLO")
)]
#[case::to_uppercase_mixed_case(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "HeLLo WoRLd",
    Ok("HELLO WORLD")
)]
#[case::to_uppercase_already_uppercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "HELLO",
    Ok("HELLO")
)]
#[case::to_uppercase_with_numbers(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "test123",
    Ok("TEST123")
)]
#[case::to_uppercase_with_special_chars(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "hello@world.com",
    Ok("HELLO@WORLD.COM")
)]
#[case::to_uppercase_unicode(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "café",
    Ok("CAFÉ")
)]
#[case::to_uppercase_empty_string(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "",
    Ok("")
)]
// ============================================================================
// ToLowerCase Transformation Tests
// ============================================================================
#[case::to_lowercase_simple_uppercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase)
        ],
    }),
    "HELLO",
    Ok("hello")
)]
#[case::to_lowercase_mixed_case(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase)
        ],
    }),
    "HeLLo WoRLd",
    Ok("hello world")
)]
#[case::to_lowercase_already_lowercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase)
        ],
    }),
    "hello",
    Ok("hello")
)]
#[case::to_lowercase_with_numbers(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase)
        ],
    }),
    "TEST123",
    Ok("test123")
)]
#[case::to_lowercase_with_special_chars(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase)
        ],
    }),
    "HELLO@WORLD.COM",
    Ok("hello@world.com")
)]
#[case::to_lowercase_unicode(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase)
        ],
    }),
    "CAFÉ",
    Ok("café")
)]
#[case::to_lowercase_empty_string(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase)
        ],
    }),
    "",
    Ok("")
)]
// ============================================================================
// StripPrefix Transformation Tests - Success Cases
// ============================================================================
#[case::strip_prefix_simple(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "Mr. ".to_string(),
            })
        ],
    }),
    "Mr. John Doe",
    Ok("John Doe")
)]
#[case::strip_prefix_with_space(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "  ".to_string(),
            })
        ],
    }),
    "  Hello",
    Ok("Hello")
)]
#[case::strip_prefix_number(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "123-".to_string(),
            })
        ],
    }),
    "123-456-7890",
    Ok("456-7890")
)]
#[case::strip_prefix_empty_result(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "PREFIX".to_string(),
            })
        ],
    }),
    "PREFIX",
    Ok("")
)]
// ============================================================================
// StripPrefix Transformation Tests - Error Cases
// ============================================================================
#[case::strip_prefix_not_found(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "Mr. ".to_string(),
            })
        ],
    }),
    "John Doe",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("John Doe".to_string()),
            rule_error: RuleError::PrefixNotFound {
                prefix: "Mr. ".to_string(),
            },
        },
    })
)]
#[case::strip_prefix_partial_match(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "Mr. ".to_string(),
            })
        ],
    }),
    "Mr John",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("Mr John".to_string()),
            rule_error: RuleError::PrefixNotFound {
                prefix: "Mr. ".to_string(),
            },
        },
    })
)]
#[case::strip_prefix_case_sensitive(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "Mr. ".to_string(),
            })
        ],
    }),
    "mr. john",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("mr. john".to_string()),
            rule_error: RuleError::PrefixNotFound {
                prefix: "Mr. ".to_string(),
            },
        },
    })
)]
// ============================================================================
// StripSuffix Transformation Tests - Success Cases
// ============================================================================
#[case::strip_suffix_simple(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: " Jr.".to_string(),
            })
        ],
    }),
    "John Doe Jr.",
    Ok("John Doe")
)]
#[case::strip_suffix_with_space(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: "  ".to_string(),
            })
        ],
    }),
    "Hello  ",
    Ok("Hello")
)]
#[case::strip_suffix_extension(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: ".txt".to_string(),
            })
        ],
    }),
    "file.txt",
    Ok("file")
)]
#[case::strip_suffix_empty_result(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: "SUFFIX".to_string(),
            })
        ],
    }),
    "SUFFIX",
    Ok("")
)]
// ============================================================================
// StripSuffix Transformation Tests - Error Cases
// ============================================================================
#[case::strip_suffix_not_found(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: " Jr.".to_string(),
            })
        ],
    }),
    "John Doe",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("John Doe".to_string()),
            rule_error: RuleError::SuffixNotFound {
                suffix: " Jr.".to_string(),
            },
        },
    })
)]
#[case::strip_suffix_partial_match(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: " Jr.".to_string(),
            })
        ],
    }),
    "John Jr",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("John Jr".to_string()),
            rule_error: RuleError::SuffixNotFound {
                suffix: " Jr.".to_string(),
            },
        },
    })
)]
#[case::strip_suffix_case_sensitive(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: " Jr.".to_string(),
            })
        ],
    }),
    "john jr.",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("john jr.".to_string()),
            rule_error: RuleError::SuffixNotFound {
                suffix: " Jr.".to_string(),
            },
        },
    })
)]
// ============================================================================
// Combined Transformations Tests
// ============================================================================
#[case::combined_strip_prefix_then_uppercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "Mr. ".to_string(),
            }),
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase),
        ],
    }),
    "Mr. john",
    Ok("JOHN")
)]
#[case::combined_strip_suffix_then_lowercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: ".TXT".to_string(),
            }),
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase),
        ],
    }),
    "HELLO.TXT",
    Ok("hello")
)]
#[case::combined_strip_prefix_suffix_uppercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "Mr. ".to_string(),
            }),
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: " Jr.".to_string(),
            }),
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase),
        ],
    }),
    "Mr. John Jr.",
    Ok("JOHN")
)]
#[case::combined_uppercase_then_lowercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase),
            StringTransformationRule::V1(StringTransformationRuleV1::ToLowerCase),
        ],
    }),
    "HeLLo",
    Ok("hello")
)]
// ============================================================================
// ReplaceChar Transformation Tests
// ============================================================================
#[case::replace_char_all_delete(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: None,
                mode: ReplaceMode::All,
            })
        ],
    }),
    "TXN-2026-00123",
    Ok("TXN202600123")
)]
#[case::replace_char_all_replace(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '/',
                to: Some('_'),
                mode: ReplaceMode::All,
            })
        ],
    }),
    "a/b/c",
    Ok("a_b_c")
)]
#[case::replace_char_single_from_start(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: Some('_'),
                mode: ReplaceMode::Single { occurrence: 1, from_end: false },
            })
        ],
    }),
    "a-b-c",
    Ok("a-b_c")
)]
#[case::replace_char_single_from_end(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '/',
                to: Some('_'),
                mode: ReplaceMode::Single { occurrence: 0, from_end: true },
            })
        ],
    }),
    "a/b/c",
    Ok("a/b_c")
)]
#[case::replace_char_single_out_of_range_noop(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: Some('_'),
                mode: ReplaceMode::Single { occurrence: 5, from_end: false },
            })
        ],
    }),
    "a-b",
    Ok("a-b")
)]
#[case::replace_char_absent_char_noop(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: None,
                mode: ReplaceMode::All,
            })
        ],
    }),
    "TXN202600123",
    Ok("TXN202600123")
)]
#[case::replace_char_empty_string_noop(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: None,
                mode: ReplaceMode::All,
            })
        ],
    }),
    "",
    Ok("")
)]
#[case::replace_char_multibyte_input(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: Some('_'),
                mode: ReplaceMode::Single { occurrence: 0, from_end: false },
            })
        ],
    }),
    "café-au-lait",
    Ok("café_au-lait")
)]
#[case::replace_char_after_trim(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::Trim),
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: None,
                mode: ReplaceMode::All,
            }),
        ],
    }),
    "  TXN-2026-00123  ",
    Ok("TXN202600123")
)]
#[case::replace_char_then_uppercase(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: None,
                mode: ReplaceMode::All,
            }),
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase),
        ],
    }),
    "txn-001",
    Ok("TXN001")
)]
#[case::replace_char_then_validate_max_length_pass(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![
            StringValidationRule::V1(StringValidationRuleV1::MaxLength { value: 12 })
        ],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: None,
                mode: ReplaceMode::All,
            })
        ],
    }),
    "TXN-2026-00123",
    Ok("TXN202600123")
)]
#[case::replace_char_then_validate_max_length_fail(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![
            StringValidationRule::V1(StringValidationRuleV1::MaxLength { value: 12 })
        ],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ReplaceChar {
                from: '-',
                to: None,
                mode: ReplaceMode::All,
            })
        ],
    }),
    "TXN-2026-001234",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("TXN2026001234".to_string()),
            rule_error: RuleError::ValueTooLong {
                max_length: "12".to_string(),
            },
        },
    })
)]
// ============================================================================
// Transformation + Validation Tests - Success Cases
// ============================================================================
#[case::uppercase_then_validate_max_length_pass(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![
            StringValidationRule::V1(StringValidationRuleV1::MaxLength { value: 10 })
        ],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "hello",
    Ok("HELLO")
)]
#[case::strip_prefix_then_validate_min_length_pass(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![
            StringValidationRule::V1(StringValidationRuleV1::MinLength { value: 2 })
        ],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "PREFIX_".to_string(),
            })
        ],
    }),
    "PREFIX_hello",
    Ok("hello")
)]
#[case::combined_transform_then_validate_pass(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![
            StringValidationRule::V1(StringValidationRuleV1::MinLength { value: 3 }),
            StringValidationRule::V1(StringValidationRuleV1::MaxLength { value: 10 }),
        ],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "Mr. ".to_string(),
            }),
            StringTransformationRule::V1(StringTransformationRuleV1::StripSuffix {
                suffix: " Jr.".to_string(),
            }),
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase),
        ],
    }),
    "Mr. John Jr.",
    Ok("JOHN")
)]
// ============================================================================
// Transformation + Validation Tests - Failure Cases
// ============================================================================
#[case::uppercase_then_validate_max_length_fail(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![
            StringValidationRule::V1(StringValidationRuleV1::MaxLength { value: 5 })
        ],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::ToUpperCase)
        ],
    }),
    "this is too long",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("THIS IS TOO LONG".to_string()),
            rule_error: RuleError::ValueTooLong {
                max_length: "5".to_string(),
            },
        },
    })
)]
#[case::strip_prefix_then_validate_min_length_fail(
    StringConfig::V1(StringConfigV1 {
        validation_rules: vec![
            StringValidationRule::V1(StringValidationRuleV1::MinLength { value: 10 })
        ],
        transformation_rules: vec![
            StringTransformationRule::V1(StringTransformationRuleV1::StripPrefix {
                prefix: "PREFIX_".to_string(),
            })
        ],
    }),
    "PREFIX_hi",
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::RuleError {
            actual_value: Some("hi".to_string()),
            rule_error: RuleError::ValueTooShort {
                min_length: "10".to_string(),
            },
        },
    })
)]
fn test_string_transformations_and_validations(
    #[case] string_config: StringConfig,
    #[case] input: &str,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .add_metadata_field(
            "test_field",
            "test_field",
            FieldTypeConfig::V1(FieldTypeConfigV1::String(string_config)),
            true,
        )
        .build();

    let result = validate_single_field(&schema, "test_field", input);

    match expected {
        Ok(expected_value) => {
            assert!(
                result.is_ok(),
                "Expected validation to succeed, but got error: {:?}",
                result
            );
            assert_eq!(result.unwrap(), expected_value);
        }
        Err(expected_error) => {
            assert!(
                result.is_err(),
                "Expected validation to fail, but got success: {:?}",
                result
            );
            let actual_error = result.unwrap_err();
            assert_field_error_matches(&actual_error, &expected_error);
        }
    }
}
