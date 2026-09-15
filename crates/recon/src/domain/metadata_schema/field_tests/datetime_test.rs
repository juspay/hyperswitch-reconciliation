//! DateTime field validation tests

use common_types::metadata_schema::{
    datatypes::datetime::{
        DateFormatConfig, DateOrder, DateTimeConfig, DateTimeConfigV2, DateTimeFormat, Delimiter,
        TimeFormatConfig, TimeOrder,
    },
    FieldTypeConfig, FieldTypeConfigV1,
};
use rstest::rstest;

use super::{
    base_schema::BaseSchemaBuilder,
    helpers::{assert_field_error_matches, validate_single_field},
};
use crate::domain::metadata_schema::types::{FieldError, ValueError};

#[rstest]
// DayMonthYear with different delimiters - success
#[case::dmy_hyphen_valid(
    "15-03-2024",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 0:00:00.0")
)]
#[case::dmy_slash_valid(
    "15/03/2024",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Slash,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 0:00:00.0")
)]
#[case::dmy_dot_valid(
    "15.03.2024",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Dot,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 0:00:00.0")
)]
// MonthDayYear format - success
#[case::mdy_valid(
    "03-15-2024",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::MonthDayYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 0:00:00.0")
)]
// YearMonthDay format - success
#[case::ymd_valid(
    "2024-03-15",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::YearMonthDay,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 0:00:00.0")
)]
// YearDayMonth format - success
#[case::ydm_valid(
    "2024-15-03",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::YearDayMonth,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 0:00:00.0")
)]
// Date with 24-hour time (with seconds) - success
#[case::date_time_24h_with_seconds(
    "15-03-2024 14:30:45",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: Some(TimeFormatConfig {
                order: TimeOrder::TwentyFourHourWithSeconds,
                delimiter: Delimiter::Colon,
                separator_from_date: Delimiter::Space,
                fractional_seconds: None,
                timezone: None,
            }),
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 14:30:45.0")
)]
// Date with 24-hour time (without seconds) - success
#[case::date_time_24h_without_seconds(
    "15-03-2024 14:30",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: Some(TimeFormatConfig {
                order: TimeOrder::TwentyFourHourWithoutSeconds,
                delimiter: Delimiter::Colon,
                separator_from_date: Delimiter::Space,
                fractional_seconds: None,
                timezone: None,
            }),
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 14:30:00.0")
)]
// Date with 12-hour time (with seconds and AM/PM) - success
#[case::date_time_12h_with_seconds_pm(
    "15-03-2024 2:30:45 PM",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: Some(TimeFormatConfig {
                order: TimeOrder::TwelveHourWithSeconds,
                delimiter: Delimiter::Colon,
                separator_from_date: Delimiter::Space,
                fractional_seconds: None,
                timezone: None,
            }),
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 2:30:45.0")
)]
#[case::date_time_12h_with_seconds_am(
    "15-03-2024 2:30:45 AM",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: Some(TimeFormatConfig {
                order: TimeOrder::TwelveHourWithSeconds,
                delimiter: Delimiter::Colon,
                separator_from_date: Delimiter::Space,
                fractional_seconds: None,
                timezone: None,
            }),
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 2:30:45.0")
)]
// Date with 12-hour time (without seconds) - success
#[case::date_time_12h_without_seconds(
    "15-03-2024 2:30 PM",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: Some(TimeFormatConfig {
                order: TimeOrder::TwelveHourWithoutSeconds,
                delimiter: Delimiter::Colon,
                separator_from_date: Delimiter::Space,
                fractional_seconds: None,
                timezone: None,
            }),
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-15 2:30:00.0")
)]
// Edge cases - success
#[case::single_digit_day_month(
    "5-3-2024",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-03-05 0:00:00.0")
)]
#[case::leap_year_feb_29(
    "29-02-2024",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2024-02-29 0:00:00.0")
)]
// MonthAbbrDayYear format - success
#[case::month_abbr_day_year_valid(
    "Mar 20, 2026",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::AbbrMonthSpaceDayCommaYear,
                delimiter: Delimiter::None,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Ok("2026-03-20 0:00:00.0")
)]
// Invalid format cases
#[case::wrong_delimiter(
    "15/03/2024",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidFormat {
            actual_value: Some("15/03/2024".to_string()),
        },
    })
)]
#[case::invalid_date(
    "32-13-2024",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidFormat {
            actual_value: Some("32-13-2024".to_string()),
        },
    })
)]
#[case::invalid_feb_29_non_leap_year(
    "29-02-2023",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidFormat {
            actual_value: Some("29-02-2023".to_string()),
        },
    })
)]
#[case::empty_string(
    "",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidFormat {
            actual_value: Some("".to_string()),
        },
    })
)]
#[case::not_a_date(
    "abc",
    DateTimeConfig::V2(DateTimeConfigV2 {
        format: DateTimeFormat {
            date_format: DateFormatConfig {
                order: DateOrder::DayMonthYear,
                delimiter: Delimiter::Hyphen,
            },
            time_format: None,
        },
        transformation_rules: vec![],
        post_parse_transformations: vec![],
    }),
    Err(FieldError::ValueError {
        identifier: "test_field".to_string(),
        error: ValueError::InvalidFormat {
            actual_value: Some("abc".to_string()),
        },
    })
)]
fn test_datetime_validations(
    #[case] input: &str,
    #[case] datetime_config: DateTimeConfig,
    #[case] expected: Result<&str, FieldError>,
) {
    let schema = BaseSchemaBuilder::new()
        .with_default_main_fields()
        .add_metadata_field(
            "test_field",
            "test_field",
            FieldTypeConfig::V1(FieldTypeConfigV1::DateTime(datetime_config)),
            true,
        )
        .build();

    let result = validate_single_field(&schema, "test_field", input);

    match (result, expected) {
        (Ok(actual), Ok(expected)) => {
            assert_eq!(actual, expected, "DateTime values don't match");
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
