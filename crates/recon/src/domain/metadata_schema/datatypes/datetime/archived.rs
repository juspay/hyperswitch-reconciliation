use common_types::metadata_schema::datatypes::datetime::archived::DateTimeConfigV1;
use error_stack::Report;
use time::{macros::format_description, Date, PrimitiveDateTime, Time};

use crate::domain::metadata_schema::{
    data_validator::ValueValidator, types::ConfigFieldValidationError, ConfigValidator, ValueError,
};

impl ValueValidator for DateTimeConfigV1 {
    type DataType = PrimitiveDateTime;
    fn validate_value(
        &self,
        value: Option<String>,
    ) -> Result<Option<Self::DataType>, Report<ValueError>> {
        value
            .map(|date_time_str| {
                if let Ok(date_time) = PrimitiveDateTime::parse(
                    &date_time_str,
                    &format_description!("[day]-[month]-[year] [hour]:[minute]:[second]"),
                ) {
                    return Ok(date_time);
                }
                if let Ok(date) =
                    Date::parse(&date_time_str, &format_description!("[day]-[month]-[year]"))
                {
                    return Ok(PrimitiveDateTime::new(date, Time::MIDNIGHT));
                }
                Err(Report::new(ValueError::InvalidFormat {
                    actual_value: Some(date_time_str),
                }))
            })
            .transpose()
    }
}

impl ConfigValidator for DateTimeConfigV1 {
    type ErrorType = ConfigFieldValidationError;
    fn validate_config(&self) -> Result<(), Report<Self::ErrorType>> {
        // No specific validation needed for datetime config
        Ok(())
    }
}
