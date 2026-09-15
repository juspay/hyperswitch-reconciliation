//! DateTime datatype validation implementations
//!
//! This module contains the ValueValidator implementation for PrimitiveDateTime types.

use common_types::metadata_schema::datatypes::datetime::{
    DateTimeConfig, DateTimeConfigV2, DateTimePostParseTransformationRule,
    DateTimePostParseTransformationRuleV1, DateTimeTransformationRule,
    DateTimeTransformationRuleV1,
};
use error_stack::{Report, ResultExt};
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

pub mod archived;

use crate::domain::metadata_schema::{
    config_validator::ConfigValidator,
    data_validator::{PostParseTransformer, ValueTransformer, ValueValidator},
    transformations::{self, regex_extract::validate_regex_config},
    types::{ConfigFieldValidationError, RuleError, ValueError},
};

impl ValueValidator for DateTimeConfig {
    type DataType = PrimitiveDateTime;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        match self {
            Self::V1(datetime_config_v1) => datetime_config_v1.validate_value(value),
            Self::V2(datetime_config_v2) => datetime_config_v2.validate_value(value),
        }
    }
}

impl ValueValidator for DateTimeConfigV2 {
    type DataType = PrimitiveDateTime;
    fn validate_value(
        &self,
        mut value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        for transformation in &self.transformation_rules {
            transformation
                .transform_value(&mut value)
                .map_err(|e| ValueError::from_rule_error(e, value.clone()))?;
        }

        let format_desc =
            self.build_format_description()
                .change_context(ValueError::InvalidFormat {
                    actual_value: value.clone(),
                })?;

        let has_offset = self
            .format
            .time_format
            .as_ref()
            .and_then(|t| t.timezone.as_ref())
            .is_some();

        value
            .map(|date_time_str| {
                let parsed = match (&self.format.time_format, has_offset) {
                    (Some(_), true) => {
                        OffsetDateTime::parse(&date_time_str, &format_desc).map(|odt| {
                            let utc = odt.to_offset(UtcOffset::UTC);
                            PrimitiveDateTime::new(utc.date(), utc.time())
                        })
                    }
                    (Some(_), false) => PrimitiveDateTime::parse(&date_time_str, &format_desc),
                    (None, _) => Date::parse(&date_time_str, &format_desc)
                        .map(|date| PrimitiveDateTime::new(date, Time::MIDNIGHT)),
                };

                let mut parsed_datetime = parsed.change_context(ValueError::InvalidFormat {
                    actual_value: Some(date_time_str.clone()),
                })?;

                for post_parse_rule in &self.post_parse_transformations {
                    post_parse_rule
                        .apply_post_parse_transformation(&mut parsed_datetime)
                        .map_err(|e| ValueError::from_rule_error(e, Some(date_time_str.clone())))?;
                }

                Ok(parsed_datetime)
            })
            .transpose()
    }
}

impl ValueTransformer for DateTimeTransformationRule {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(rule_v1) => rule_v1.transform_value(value),
        }
    }
}

impl ValueTransformer for DateTimeTransformationRuleV1 {
    type DataType = String;
    fn transform_value(&self, value: &mut Option<Self::DataType>) -> Result<(), Report<RuleError>> {
        match self {
            Self::Trim => {
                if let Some(inner) = value {
                    *value = Some(inner.trim().to_string());
                }
                Ok(())
            }
            Self::JsonExtract { pointer } => {
                transformations::json_extract::json_extract_value(value, pointer)
            }
            Self::Regex { pattern, group } => {
                transformations::regex_extract::regex_extract_value(value, pattern, *group)
            }
        }
    }
}

impl PostParseTransformer for DateTimePostParseTransformationRule {
    type DataType = PrimitiveDateTime;
    fn apply_post_parse_transformation(
        &self,
        value: &mut Self::DataType,
    ) -> Result<(), Report<RuleError>> {
        match self {
            Self::V1(rule_v1) => rule_v1.apply_post_parse_transformation(value),
        }
    }
}

impl PostParseTransformer for DateTimePostParseTransformationRuleV1 {
    type DataType = PrimitiveDateTime;
    fn apply_post_parse_transformation(
        &self,
        value: &mut Self::DataType,
    ) -> Result<(), Report<RuleError>> {
        match self {
            Self::Truncate { precision } => {
                transformations::datetime_post_parse::apply_truncation(value, precision)
            }
            Self::AddDuration { duration } => {
                transformations::datetime_post_parse::apply_duration_addition(value, duration)
            }
            Self::SubtractDuration { duration } => {
                transformations::datetime_post_parse::apply_duration_subtraction(value, duration)
            }
        }
    }
}

// DateTime config validators
impl ConfigValidator for DateTimeConfig {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(config_v1) => config_v1.validate_config(),
            Self::V2(config_v2) => config_v2.validate_config(),
        }
    }
}

impl ConfigValidator for DateTimeConfigV2 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        self.build_format_description()
            .change_context(ConfigFieldValidationError::InvalidDateTimeConfig)?;

        self.transformation_rules
            .iter()
            .try_for_each(|rule| rule.validate_config())?;

        Ok(())
    }
}

impl ConfigValidator for DateTimeTransformationRule {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::V1(rule_v1) => rule_v1.validate_config(),
        }
    }
}

impl ConfigValidator for DateTimeTransformationRuleV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        match self {
            Self::Trim | Self::JsonExtract { .. } => Ok(()),
            Self::Regex { pattern, group } => validate_regex_config(pattern, *group),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::domain::metadata_schema::data_validator::ValueValidator;
    use common_types::metadata_schema::datatypes::datetime::{
        DateFormatConfig, DateOrder, DateTimeConfigV2, DateTimeDuration, DateTimeFormat,
        DateTimePostParseTransformationRule, DateTimePostParseTransformationRuleV1,
        DateTimeTransformationRule, DateTimeTransformationRuleV1, Delimiter, DurationUnit,
        FractionalSeconds, TimeFormatConfig, TimeOrder, TimezoneFormat, TruncationPrecision,
    };
    use time::macros::datetime;

    fn trim_rule() -> DateTimeTransformationRule {
        DateTimeTransformationRule::V1(DateTimeTransformationRuleV1::Trim)
    }

    fn date_only_config(transformation_rules: Vec<DateTimeTransformationRule>) -> DateTimeConfigV2 {
        DateTimeConfigV2 {
            format: DateTimeFormat {
                date_format: DateFormatConfig {
                    order: DateOrder::YearMonthDay,
                    delimiter: Delimiter::Hyphen,
                },
                time_format: None,
            },
            transformation_rules,
            post_parse_transformations: vec![],
        }
    }

    #[test]
    fn test_datetime_trim_before_parsing() {
        let config = date_only_config(vec![trim_rule()]);
        let result = config
            .validate_value(Some("  2024-01-15  ".to_string()))
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_datetime_without_trim_fails_on_padded_input() {
        let config = date_only_config(vec![]);
        let result = config.validate_value(Some("  2024-01-15  ".to_string()));
        assert!(result.is_err());
    }

    fn v2(time_format: Option<TimeFormatConfig>) -> DateTimeConfigV2 {
        DateTimeConfigV2 {
            format: DateTimeFormat {
                date_format: DateFormatConfig {
                    order: DateOrder::YearMonthDay,
                    delimiter: Delimiter::Hyphen,
                },
                time_format,
            },
            transformation_rules: vec![],
            post_parse_transformations: vec![],
        }
    }

    #[test]
    fn parses_space_separated_with_fractional_seconds() {
        let config = v2(Some(TimeFormatConfig {
            order: TimeOrder::TwentyFourHourWithSeconds,
            delimiter: Delimiter::Colon,
            separator_from_date: Delimiter::Space,
            fractional_seconds: Some(FractionalSeconds::Microseconds {
                separator: Delimiter::Dot,
            }),
            timezone: None,
        }));
        let parsed = config
            .validate_value(Some("2026-01-24 14:43:57.775586".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-01-24 14:43:57.775586));
    }

    #[test]
    fn parses_iso8601_with_offset_and_normalizes_to_utc() {
        let config = v2(Some(TimeFormatConfig {
            order: TimeOrder::TwentyFourHourWithSeconds,
            delimiter: Delimiter::Colon,
            separator_from_date: Delimiter::TLiteral,
            fractional_seconds: Some(FractionalSeconds::Microseconds {
                separator: Delimiter::Dot,
            }),
            timezone: Some(TimezoneFormat::OffsetHourMinute),
        }));
        let parsed = config
            .validate_value(Some("2026-03-25T03:50:40.677235+11:00".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-24 16:50:40.677235));
    }

    #[test]
    fn backward_compatible_with_prior_config_shape() {
        let config = v2(Some(TimeFormatConfig {
            order: TimeOrder::TwentyFourHourWithSeconds,
            delimiter: Delimiter::Colon,
            separator_from_date: Delimiter::Space,
            fractional_seconds: None,
            timezone: None,
        }));
        let parsed = config
            .validate_value(Some("2026-03-23 15:17:30".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-23 15:17:30));
    }

    fn datetime_config(
        transformation_rules: Vec<DateTimeTransformationRule>,
        post_parse_transformations: Vec<DateTimePostParseTransformationRule>,
    ) -> DateTimeConfigV2 {
        DateTimeConfigV2 {
            format: DateTimeFormat {
                date_format: DateFormatConfig {
                    order: DateOrder::YearMonthDay,
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
            transformation_rules,
            post_parse_transformations,
        }
    }

    fn truncate(precision: TruncationPrecision) -> DateTimePostParseTransformationRule {
        DateTimePostParseTransformationRule::V1(DateTimePostParseTransformationRuleV1::Truncate {
            precision,
        })
    }

    fn add_duration(value: u32, unit: DurationUnit) -> DateTimePostParseTransformationRule {
        DateTimePostParseTransformationRule::V1(
            DateTimePostParseTransformationRuleV1::AddDuration {
                duration: DateTimeDuration { value, unit },
            },
        )
    }

    fn sub_duration(value: u32, unit: DurationUnit) -> DateTimePostParseTransformationRule {
        DateTimePostParseTransformationRule::V1(
            DateTimePostParseTransformationRuleV1::SubtractDuration {
                duration: DateTimeDuration { value, unit },
            },
        )
    }

    #[test]
    fn truncate_start_of_hour_zeroes_minutes_and_seconds() {
        let config = datetime_config(vec![], vec![truncate(TruncationPrecision::StartOfHour)]);
        let parsed = config
            .validate_value(Some("2026-03-25 14:30:45".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-25 14:00:00));
    }

    #[test]
    fn truncate_start_of_day_zeroes_time_component() {
        let config = datetime_config(vec![], vec![truncate(TruncationPrecision::StartOfDay)]);
        let parsed = config
            .validate_value(Some("2026-03-25 14:30:45".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-25 00:00:00));
    }

    #[test]
    fn truncate_start_of_month_resets_day_and_time() {
        let config = datetime_config(vec![], vec![truncate(TruncationPrecision::StartOfMonth)]);
        let parsed = config
            .validate_value(Some("2026-03-25 14:30:45".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-01 00:00:00));
    }

    #[test]
    fn truncate_start_of_year_resets_month_day_and_time() {
        let config = datetime_config(vec![], vec![truncate(TruncationPrecision::StartOfYear)]);
        let parsed = config
            .validate_value(Some("2026-03-25 14:30:45".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-01-01 00:00:00));
    }

    #[test]
    fn add_duration_hours_rolls_over_day() {
        let config = datetime_config(vec![], vec![add_duration(1, DurationUnit::Hours)]);
        let parsed = config
            .validate_value(Some("2026-03-25 23:45:00".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-26 00:45:00));
    }

    #[test]
    fn subtract_duration_days_rolls_back_month() {
        let config = datetime_config(vec![], vec![sub_duration(1, DurationUnit::Days)]);
        let parsed = config
            .validate_value(Some("2026-04-01 08:00:00".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-31 08:00:00));
    }

    #[test]
    fn composable_hour_grain_join_key_settlement_side() {
        // Settlement side: creation hour + 1, truncated to top of hour.
        let config = datetime_config(
            vec![],
            vec![
                add_duration(1, DurationUnit::Hours),
                truncate(TruncationPrecision::StartOfHour),
            ],
        );
        let parsed = config
            .validate_value(Some("2026-03-25 10:05:14".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-25 11:00:00));
    }

    #[test]
    fn composable_hour_grain_join_key_balance_side() {
        // Balance side: booking time truncated to top of hour.
        let config = datetime_config(vec![], vec![truncate(TruncationPrecision::StartOfHour)]);
        let parsed = config
            .validate_value(Some("2026-03-25 11:00:00".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-25 11:00:00));
    }

    #[test]
    fn pre_parse_trim_composes_with_post_parse_truncate() {
        let config = datetime_config(
            vec![trim_rule()],
            vec![truncate(TruncationPrecision::StartOfHour)],
        );
        let parsed = config
            .validate_value(Some("  2026-03-25 14:30:45  ".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-25 14:00:00));
    }

    fn regex_rule(pattern: &str, group: Option<usize>) -> DateTimeTransformationRule {
        DateTimeTransformationRule::V1(DateTimeTransformationRuleV1::Regex {
            pattern: pattern.to_string(),
            group,
        })
    }

    #[test]
    fn regex_extracts_date_from_surrounding_text() {
        let config = DateTimeConfigV2 {
            format: DateTimeFormat {
                date_format: DateFormatConfig {
                    order: DateOrder::DayMonthYear,
                    delimiter: Delimiter::Slash,
                },
                time_format: None,
            },
            transformation_rules: vec![regex_rule(r"(\d{2}/\d{2}/\d{4})", Some(1))],
            post_parse_transformations: vec![],
        };
        let parsed = config
            .validate_value(Some("Totals for Payment on 22/06/2026".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-06-22 00:00:00));
    }

    #[test]
    fn regex_full_match_without_group_extracts_date() {
        let config = date_only_config(vec![regex_rule(r"\d{4}-\d{2}-\d{2}", None)]);
        let parsed = config
            .validate_value(Some("settlement on 2026-03-25 (batch 12)".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-25 00:00:00));
    }

    #[test]
    fn regex_extracts_datetime_with_time_component() {
        let config = datetime_config(
            vec![regex_rule(
                r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})",
                Some(1),
            )],
            vec![],
        );
        let parsed = config
            .validate_value(Some("Booked at 2026-03-25 14:30:45 UTC".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-25 14:30:45));
    }

    #[test]
    fn regex_composes_with_post_parse_truncate() {
        let config = datetime_config(
            vec![regex_rule(
                r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})",
                Some(1),
            )],
            vec![truncate(TruncationPrecision::StartOfHour)],
        );
        let parsed = config
            .validate_value(Some("Creation Date: 2026-03-25 14:30:45".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-25 14:00:00));
    }

    #[test]
    fn regex_no_match_errors() {
        let config = date_only_config(vec![regex_rule(r"\d{4}-\d{2}-\d{2}", None)]);
        let result = config.validate_value(Some("no date in here".into()));
        assert!(result.is_err());
    }

    #[test]
    fn regex_none_value_passes_through() {
        let config = date_only_config(vec![regex_rule(r"\d{4}-\d{2}-\d{2}", None)]);
        let result = config.validate_value(None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn regex_extracted_value_still_parsed_against_configured_format() {
        let config = date_only_config(vec![regex_rule(r"(\d{2}/\d{2}/\d{4})", Some(1))]);
        let result = config.validate_value(Some("Totals for Payment on 22/06/2026".into()));
        assert!(result.is_err());
    }

    #[test]
    fn regex_invalid_pattern_rejected_at_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = date_only_config(vec![regex_rule("[", None)]);
        assert!(config.validate_config().is_err());
    }

    #[test]
    fn regex_group_out_of_range_rejected_at_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = date_only_config(vec![regex_rule(r"(\d{4})", Some(2))]);
        assert!(config.validate_config().is_err());
    }

    #[test]
    fn regex_valid_pattern_passes_config_validation() {
        use crate::domain::metadata_schema::config_validator::ConfigValidator;
        let config = date_only_config(vec![regex_rule(r"(\d{4}-\d{2}-\d{2})", Some(1))]);
        assert!(config.validate_config().is_ok());
    }

    #[test]
    fn regex_json_config_deserializes_and_extracts() {
        let json = serde_json::json!({
            "date_time_config_version": "v2",
            "format": {
                "date_format": { "order": "day_month_year", "delimiter": "slash" },
                "time_format": null
            },
            "transformation_rules": [{
                "date_time_transformation_rule_version": "v1",
                "date_time_transformation_rule_type": "regex",
                "pattern": "(\\d{2}/\\d{2}/\\d{4})",
                "group": 1
            }]
        });
        let config: common_types::metadata_schema::datatypes::datetime::DateTimeConfig =
            serde_json::from_value(json).unwrap();
        let parsed = config
            .validate_value(Some("Totals for Payment on 22/06/2026".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-06-22 00:00:00));
    }

    #[test]
    fn old_json_config_deserializes() {
        let json = serde_json::json!({
            "date_time_config_version": "v2",
            "format": {
                "date_format": { "order": "year_month_day", "delimiter": "hyphen" },
                "time_format": {
                    "order": "twenty_four_hour_with_seconds",
                    "delimiter": "colon",
                    "separator_from_date": "space"
                }
            }
        });
        let config: common_types::metadata_schema::datatypes::datetime::DateTimeConfig =
            serde_json::from_value(json).unwrap();
        let parsed = config
            .validate_value(Some("2026-03-23 15:17:30".into()))
            .unwrap()
            .unwrap();
        assert_eq!(parsed, datetime!(2026-03-23 15:17:30));
    }
}
