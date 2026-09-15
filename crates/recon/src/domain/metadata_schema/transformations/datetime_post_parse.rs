use common_types::metadata_schema::datatypes::datetime::{
    DateTimeDuration, DurationUnit, TruncationPrecision,
};
use error_stack::{report, Report};
use time::{Date, Duration, PrimitiveDateTime, Time};

use crate::domain::metadata_schema::types::RuleError;

pub fn apply_truncation(
    value: &mut PrimitiveDateTime,
    precision: &TruncationPrecision,
) -> Result<(), Report<RuleError>> {
    let truncated = match precision {
        TruncationPrecision::StartOfHour => {
            value.replace_time(Time::from_hms(value.hour(), 0, 0).map_err(|e| {
                report!(RuleError::InvalidDateTimeConfig {
                    details: format!("truncate to start_of_hour failed: {e}"),
                })
            })?)
        }
        TruncationPrecision::StartOfDay => value.replace_time(Time::MIDNIGHT),
        TruncationPrecision::StartOfMonth => {
            let date = Date::from_calendar_date(value.year(), value.month(), 1).map_err(|e| {
                report!(RuleError::InvalidDateTimeConfig {
                    details: format!("truncate to start_of_month failed: {e}"),
                })
            })?;
            PrimitiveDateTime::new(date, Time::MIDNIGHT)
        }
        TruncationPrecision::StartOfYear => {
            let date =
                Date::from_calendar_date(value.year(), time::Month::January, 1).map_err(|e| {
                    report!(RuleError::InvalidDateTimeConfig {
                        details: format!("truncate to start_of_year failed: {e}"),
                    })
                })?;
            PrimitiveDateTime::new(date, Time::MIDNIGHT)
        }
    };
    *value = truncated;
    Ok(())
}

fn duration_from_config(duration: &DateTimeDuration) -> Duration {
    let magnitude = i64::from(duration.value);
    match duration.unit {
        DurationUnit::Minutes => Duration::minutes(magnitude),
        DurationUnit::Hours => Duration::hours(magnitude),
        DurationUnit::Days => Duration::days(magnitude),
    }
}

pub fn apply_duration_addition(
    value: &mut PrimitiveDateTime,
    duration: &DateTimeDuration,
) -> Result<(), Report<RuleError>> {
    let shifted = value
        .checked_add(duration_from_config(duration))
        .ok_or_else(|| {
            report!(RuleError::InvalidDateTimeConfig {
                details: format!(
                    "add_duration overflow: {} {:?}",
                    duration.value, duration.unit
                ),
            })
        })?;
    *value = shifted;
    Ok(())
}

pub fn apply_duration_subtraction(
    value: &mut PrimitiveDateTime,
    duration: &DateTimeDuration,
) -> Result<(), Report<RuleError>> {
    let shifted = value
        .checked_sub(duration_from_config(duration))
        .ok_or_else(|| {
            report!(RuleError::InvalidDateTimeConfig {
                details: format!(
                    "subtract_duration overflow: {} {:?}",
                    duration.value, duration.unit
                ),
            })
        })?;
    *value = shifted;
    Ok(())
}
