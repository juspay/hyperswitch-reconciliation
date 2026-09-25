use core::fmt;
use std::str::FromStr;

use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Text};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Default, PartialEq, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
#[schema(value_type = String, example = "Asia/Kolkata")]
pub struct Timezone(chrono_tz::Tz);

impl Timezone {
    pub const UTC: Self = Self(chrono_tz::Tz::UTC);
}

impl fmt::Display for Timezone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.name())
    }
}

impl FromStr for Timezone {
    type Err = TimezoneParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse::<chrono_tz::Tz>()
            .map(Self)
            .map_err(|_| TimezoneParseError::InvalidTimezone {
                value: value.to_owned(),
            })
    }
}

/// Returned when a string is not a name the bundled tzdb recognises.
#[derive(Debug, thiserror::Error)]
pub enum TimezoneParseError {
    #[error("`{value}` is not a valid IANA timezone name")]
    InvalidTimezone { value: String },
}

impl Serialize for Timezone {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.name())
    }
}

impl<'de> Deserialize<'de> for Timezone {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(de::Error::custom)
    }
}

common_utils::impl_to_sql_from_sql_text!(Timezone);
