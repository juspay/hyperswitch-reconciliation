use core::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::metadata_schema::datatypes::datetime::archived::DateTimeConfigV1;

pub mod archived;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "date_time_config_version", rename_all = "snake_case")]
pub enum DateTimeConfig {
    V1(DateTimeConfigV1),
    V2(DateTimeConfigV2),
}

/// DateTime configuration V2
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DateTimeConfigV2 {
    pub format: DateTimeFormat,
    #[serde(default)]
    pub transformation_rules: Vec<DateTimeTransformationRule>,
    #[serde(default)]
    pub post_parse_transformations: Vec<DateTimePostParseTransformationRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(
    tag = "date_time_transformation_rule_version",
    rename_all = "snake_case"
)]
pub enum DateTimeTransformationRule {
    V1(DateTimeTransformationRuleV1),
}

/// Pre-parse transformation rules. Operate on the raw `Option<String>` before `PrimitiveDateTime::parse`.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "date_time_transformation_rule_type", rename_all = "snake_case")]
pub enum DateTimeTransformationRuleV1 {
    /// Strip leading/trailing whitespace from the raw string.
    Trim,
    /// Extract a datetime string from a JSON document via RFC 6901 pointer.
    JsonExtract { pointer: String },
    /// Extract the datetime substring embedded in a larger string via a regex pattern.
    Regex {
        pattern: String,
        group: Option<usize>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "date_time_post_parse_rule_version", rename_all = "snake_case")]
pub enum DateTimePostParseTransformationRule {
    V1(DateTimePostParseTransformationRuleV1),
}

/// Post-parse transformation rules. Operate on the parsed `PrimitiveDateTime` after parsing succeeds.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "date_time_post_parse_rule_type", rename_all = "snake_case")]
pub enum DateTimePostParseTransformationRuleV1 {
    /// Truncate to the start of the specified precision (hour/day/month/year).
    Truncate { precision: TruncationPrecision },
    /// Add a `DateTimeDuration` to the parsed datetime.
    AddDuration { duration: DateTimeDuration },
    /// Subtract a `DateTimeDuration` from the parsed datetime.
    SubtractDuration { duration: DateTimeDuration },
}

/// Truncation precision for datetime values. Truncation rounds **down** to the start of the
/// specified unit (minute/second/sub-second are zeroed for `StartOfHour`, etc.).
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TruncationPrecision {
    /// `2026-03-25 14:30:45.123` → `2026-03-25 14:00:00`
    StartOfHour,
    /// `2026-03-25 14:30:45.123` → `2026-03-25 00:00:00`
    StartOfDay,
    /// `2026-03-25 14:30:45.123` → `2026-03-01 00:00:00`
    StartOfMonth,
    /// `2026-03-25 14:30:45.123` → `2026-01-01 00:00:00`
    StartOfYear,
}

/// Duration value with unit for datetime arithmetic (`AddDuration` / `SubtractDuration`).
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct DateTimeDuration {
    pub value: u32,
    pub unit: DurationUnit,
}

/// Duration units supported for datetime arithmetic.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DurationUnit {
    Minutes,
    Hours,
    Days,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DateTimeFormat {
    pub date_format: DateFormatConfig,
    /// Optional time format configuration (if None, only date is expected)
    pub time_format: Option<TimeFormatConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DateFormatConfig {
    pub order: DateOrder,
    pub delimiter: Delimiter,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TimeFormatConfig {
    pub order: TimeOrder,
    pub delimiter: Delimiter,
    pub separator_from_date: Delimiter,
    /// Optional fractional seconds configuration (e.g. `.775586`).
    #[serde(default)]
    pub fractional_seconds: Option<FractionalSeconds>,
    /// When set, values are parsed as offset-aware and normalized to UTC.
    #[serde(default)]
    pub timezone: Option<TimezoneFormat>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "precision", rename_all = "snake_case")]
pub enum FractionalSeconds {
    /// 6 digits (e.g. `.775586`)
    Microseconds { separator: Delimiter },
}

impl FractionalSeconds {
    pub fn format_string(&self) -> String {
        match self {
            Self::Microseconds { separator } => format!("{separator}[subsecond digits:6]"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimezoneFormat {
    /// `+HH:MM`
    OffsetHourMinute,
}

impl TimezoneFormat {
    pub fn format_token(&self) -> &'static str {
        match self {
            Self::OffsetHourMinute => "[offset_hour sign:mandatory]:[offset_minute]",
        }
    }
}

/// Date component ordering
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DateOrder {
    /// Day-Month-Year (e.g., 31-12-2024)
    DayMonthYear,
    /// Month-Day-Year (e.g., 12-31-2024)
    MonthDayYear,
    /// Year-Month-Day (e.g., 2024-12-31)
    YearMonthDay,
    /// Year-Day-Month (e.g., 2024-31-12)
    YearDayMonth,
    /// Fixed format: `<Mon> <D>, <YYYY>` (e.g., Mar 20, 2026).
    /// The `delimiter` field in `DateFormatConfig` is not applicable for this variant.
    AbbrMonthSpaceDayCommaYear,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Delimiter {
    Colon,
    Slash,
    Hyphen,
    Dot,
    Space,
    None,
    TLiteral,
}

impl fmt::Display for Delimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Colon => write!(f, ":"),
            Self::Slash => write!(f, "/"),
            Self::Hyphen => write!(f, "-"),
            Self::Dot => write!(f, "."),
            Self::Space => write!(f, " "),
            Self::None => write!(f, ""),
            Self::TLiteral => write!(f, "T"),
        }
    }
}

/// Time structure/format
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeOrder {
    /// 24-hour format with seconds (HH:MM:SS)
    TwentyFourHourWithSeconds,
    /// 24-hour format without seconds (HH:MM)
    TwentyFourHourWithoutSeconds,
    /// 12-hour format with seconds and AM/PM (hh:MM:SS AM/PM)
    TwelveHourWithSeconds,
    /// 12-hour format without seconds and AM/PM (hh:MM AM/PM)
    TwelveHourWithoutSeconds,
}

impl DateFormatConfig {
    /// Build date format string from the configuration
    pub fn build_format_string(&self) -> String {
        let delim = self.delimiter.to_string();
        match self.order {
            DateOrder::DayMonthYear => {
                format!("[day padding:none]{delim}[month padding:none]{delim}[year]")
            }
            DateOrder::MonthDayYear => {
                format!("[month padding:none]{delim}[day padding:none]{delim}[year]")
            }
            DateOrder::YearMonthDay => {
                format!("[year]{delim}[month padding:none]{delim}[day padding:none]")
            }
            DateOrder::YearDayMonth => {
                format!("[year]{delim}[day padding:none]{delim}[month padding:none]")
            }
            DateOrder::AbbrMonthSpaceDayCommaYear => {
                "[month repr:short] [day padding:none], [year]".to_string()
            }
        }
    }
}

impl TimeFormatConfig {
    /// Build time format string from the configuration
    pub fn build_format_string(&self) -> String {
        let delim = self.delimiter.to_string();
        let subsecond = self
            .fractional_seconds
            .as_ref()
            .map(|fs| fs.format_string())
            .unwrap_or("".to_string());
        let offset = self
            .timezone
            .as_ref()
            .map(|tz| tz.format_token().to_string())
            .unwrap_or("".to_string());

        match self.order {
            TimeOrder::TwentyFourHourWithSeconds => {
                format!(
                    "[hour padding:none]{delim}[minute padding:none]{delim}[second padding:none]{subsecond}{offset}"
                )
            }
            TimeOrder::TwentyFourHourWithoutSeconds => {
                format!("[hour padding:none]{delim}[minute padding:none]{offset}")
            }
            TimeOrder::TwelveHourWithSeconds => {
                format!(
                    "[hour padding:none]{delim}[minute padding:none]{delim}[second padding:none]{subsecond} [period]{offset}"
                )
            }
            TimeOrder::TwelveHourWithoutSeconds => {
                format!("[hour padding:none]{delim}[minute padding:none] [period]{offset}")
            }
        }
    }
}

impl DateTimeConfigV2 {
    /// Build and parse the complete datetime format description from config
    pub fn build_format_description(
        &self,
    ) -> Result<time::format_description::OwnedFormatItem, time::error::InvalidFormatDescription>
    {
        self.format.build_format_description()
    }
}

impl DateTimeFormat {
    /// Build and parse the complete datetime format description from config
    pub fn build_format_description(
        &self,
    ) -> Result<time::format_description::OwnedFormatItem, time::error::InvalidFormatDescription>
    {
        let date_format = self.date_format.build_format_string();

        let format_string = match &self.time_format {
            Some(time_config) => {
                let time_format = time_config.build_format_string();
                format!(
                    "{date_format}{}{time_format}",
                    time_config.separator_from_date
                )
            }
            None => date_format,
        };

        time::format_description::parse_owned::<2>(&format_string)
    }
}
