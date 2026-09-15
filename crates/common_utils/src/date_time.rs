use time::{OffsetDateTime, PrimitiveDateTime};

pub fn now() -> PrimitiveDateTime {
    let utc_now = OffsetDateTime::now_utc();
    PrimitiveDateTime::new(utc_now.date(), utc_now.time())
}

pub fn now_unix_timestamp() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

pub fn weekdays_between(start: PrimitiveDateTime, end: PrimitiveDateTime) -> i64 {
    let start_date = start.date();
    let end_date = end.date();

    if start_date >= end_date {
        return 0;
    }

    let total_days = (end_date - start_date).whole_days();

    let full_weeks = total_days / 7;
    let mut week_days = full_weeks * 5;

    let remaining_days = total_days % 7;
    let start_weekday = i64::from(start_date.weekday().number_from_monday());

    for i in 1..=remaining_days {
        let day_number = (start_weekday + i) % 7;
        let weekday = if day_number == 0 { 7 } else { day_number };

        if weekday <= 5 {
            week_days += 1;
        }
    }

    week_days
}

pub use time::PrimitiveDateTime as CommonPrimitiveDateTime;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::as_conversions)]
mod tests {
    use super::*;
    use time::{Date, Month, Time};

    #[test]
    fn test_now_returns_primitive_datetime() {
        let result = now();

        // Check that the result is a valid PrimitiveDateTime
        assert!(result.year() >= 2024);
        assert!(result.month() as u8 >= 1 && result.month() as u8 <= 12);
        assert!(result.day() >= 1 && result.day() <= 31);
    }

    #[test]
    fn test_now_unix_timestamp_returns_positive_value() {
        let timestamp = now_unix_timestamp();

        // Unix timestamp should be positive and reasonable (after year 2020)
        assert!(timestamp > 1577836800); // January 1, 2020

        // Should be close to current time
        let now_timestamp = OffsetDateTime::now_utc().unix_timestamp();
        assert!((now_timestamp - timestamp).abs() <= 1); // Within 1 second
    }

    #[test]
    fn test_now_returns_utc_time() {
        let before = OffsetDateTime::now_utc();
        let result = now();
        let after = OffsetDateTime::now_utc();

        // The result should be between before and after
        let result_as_offset = OffsetDateTime::new_utc(result.date(), result.time());
        assert!(result_as_offset >= before);
        assert!(result_as_offset <= after);
    }

    #[test]
    fn test_weekdays_between_within_week() {
        let start = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::February, 2).unwrap(),
            Time::from_hms(0, 0, 0).unwrap(),
        );
        let end = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::February, 5).unwrap(),
            Time::from_hms(0, 0, 0).unwrap(),
        );
        assert_eq!(weekdays_between(start, end), 3);
    }

    #[test]
    fn test_weekdays_between_across_weeks() {
        let start = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::February, 4).unwrap(),
            Time::from_hms(0, 0, 0).unwrap(),
        );
        let end = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::February, 10).unwrap(),
            Time::from_hms(0, 0, 0).unwrap(),
        );
        assert_eq!(weekdays_between(start, end), 4);
    }

    #[test]
    fn test_weekdays_between_same_day() {
        let start = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::February, 2).unwrap(),
            Time::from_hms(0, 0, 0).unwrap(),
        );
        let end = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::February, 2).unwrap(),
            Time::from_hms(0, 0, 0).unwrap(),
        );
        assert_eq!(weekdays_between(start, end), 0);
    }
}
