//! Custom serialization/deserialization implementations.

/// Use the well-known ISO 8601 format when serializing and deserializing an
/// [`PrimitiveDateTime`][PrimitiveDateTime].
///
/// [PrimitiveDateTime]: ::time::PrimitiveDateTime
pub mod iso8601 {
    use serde::{ser::Error as _, Deserializer, Serialize, Serializer};
    use time::{
        format_description::FormatItem, macros::format_description, serde::iso8601,
        PrimitiveDateTime, UtcOffset,
    };

    const FORMAT: &[FormatItem<'_>] =
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");

    /// Serialize a [`PrimitiveDateTime`] using the well-known ISO 8601 format.
    pub fn serialize<S>(date_time: &PrimitiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        date_time
            .assume_utc()
            .format(&FORMAT)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }

    /// Deserialize an [`PrimitiveDateTime`] from its ISO 8601 representation.
    pub fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'a>,
    {
        iso8601::deserialize(deserializer).map(|offset_date_time| {
            let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
            PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
        })
    }

    /// Use the well-known ISO 8601 format when serializing and deserializing an
    /// [`Option<PrimitiveDateTime>`][PrimitiveDateTime].
    ///
    /// [PrimitiveDateTime]: ::time::PrimitiveDateTime
    pub mod option {
        use serde::Serialize;

        use super::*;

        /// Serialize an [`Option<PrimitiveDateTime>`] using the well-known ISO 8601 format.
        pub fn serialize<S>(
            date_time: &Option<PrimitiveDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            date_time
                .map(|date_time| date_time.assume_utc().format(&FORMAT))
                .transpose()
                .map_err(S::Error::custom)?
                .serialize(serializer)
        }

        /// Deserialize an [`Option<PrimitiveDateTime>`] from its ISO 8601 representation.
        pub fn deserialize<'a, D>(deserializer: D) -> Result<Option<PrimitiveDateTime>, D::Error>
        where
            D: Deserializer<'a>,
        {
            iso8601::option::deserialize(deserializer).map(|option_offset_date_time| {
                option_offset_date_time.map(|offset_date_time| {
                    let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
                    PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
                })
            })
        }
    }
    /// Use the well-known ISO 8601 format which is without timezone when serializing and deserializing an
    /// [`Option<PrimitiveDateTime>`][PrimitiveDateTime].
    ///
    /// [PrimitiveDateTime]: ::time::PrimitiveDateTime
    pub mod option_without_timezone {
        use serde::{de, Deserialize, Serialize};
        use time::macros::format_description;

        use super::*;

        /// Serialize an [`Option<PrimitiveDateTime>`] using the well-known ISO 8601 format which is without timezone.
        pub fn serialize<S>(
            date_time: &Option<PrimitiveDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            date_time
                .map(|date_time| {
                    let format =
                        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
                    date_time.assume_utc().format(format)
                })
                .transpose()
                .map_err(S::Error::custom)?
                .serialize(serializer)
        }

        /// Deserialize an [`Option<PrimitiveDateTime>`] from its ISO 8601 representation.
        pub fn deserialize<'a, D>(deserializer: D) -> Result<Option<PrimitiveDateTime>, D::Error>
        where
            D: Deserializer<'a>,
        {
            Option::deserialize(deserializer)?
                .map(|time_string| {
                    let format =
                        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
                    PrimitiveDateTime::parse(time_string, format).map_err(|_| {
                        de::Error::custom(format!(
                            "Failed to parse PrimitiveDateTime from {time_string}"
                        ))
                    })
                })
                .transpose()
        }
    }
}

pub mod iso8601_micros {
    use serde::{ser::Error as _, Deserializer, Serialize, Serializer};
    use time::{
        format_description::FormatItem, macros::format_description, serde::iso8601,
        PrimitiveDateTime, UtcOffset,
    };

    const FORMAT: &[FormatItem<'_>] =
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z");

    pub fn serialize<S>(date_time: &PrimitiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        date_time
            .assume_utc()
            .format(&FORMAT)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }

    pub fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'a>,
    {
        iso8601::deserialize(deserializer).map(|offset_date_time| {
            let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
            PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
        })
    }
}

/// Use the UNIX timestamp when serializing and deserializing an
/// [`PrimitiveDateTime`][PrimitiveDateTime].
///
/// [PrimitiveDateTime]: ::time::PrimitiveDateTime
pub mod timestamp {

    use serde::{Deserializer, Serialize, Serializer};
    use time::{serde::timestamp, PrimitiveDateTime, UtcOffset};

    /// Serialize a [`PrimitiveDateTime`] using UNIX timestamp.
    pub fn serialize<S>(date_time: &PrimitiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        date_time
            .assume_utc()
            .unix_timestamp()
            .serialize(serializer)
    }

    /// Deserialize an [`PrimitiveDateTime`] from UNIX timestamp.
    pub fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'a>,
    {
        timestamp::deserialize(deserializer).map(|offset_date_time| {
            let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
            PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
        })
    }

    /// Use the UNIX timestamp when serializing and deserializing an
    /// [`Option<PrimitiveDateTime>`][PrimitiveDateTime].
    ///
    /// [PrimitiveDateTime]: ::time::PrimitiveDateTime
    pub mod option {
        use serde::Serialize;

        use super::*;

        /// Serialize an [`Option<PrimitiveDateTime>`] from UNIX timestamp.
        pub fn serialize<S>(
            date_time: &Option<PrimitiveDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            date_time
                .map(|date_time| date_time.assume_utc().unix_timestamp())
                .serialize(serializer)
        }

        /// Deserialize an [`Option<PrimitiveDateTime>`] from UNIX timestamp.
        pub fn deserialize<'a, D>(deserializer: D) -> Result<Option<PrimitiveDateTime>, D::Error>
        where
            D: Deserializer<'a>,
        {
            timestamp::option::deserialize(deserializer).map(|option_offset_date_time| {
                option_offset_date_time.map(|offset_date_time| {
                    let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
                    PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
                })
            })
        }
    }
}

/// <https://github.com/serde-rs/serde/issues/994#issuecomment-316895860>
pub mod json_string {
    use serde::{
        de::{self, Deserialize, DeserializeOwned, Deserializer},
        ser::{self, Serialize, Serializer},
    };

    /// Serialize a type to json_string format
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        let j = serde_json::to_string(value).map_err(ser::Error::custom)?;
        j.serialize(serializer)
    }

    /// Deserialize a string which is in json format
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: DeserializeOwned,
        D: Deserializer<'de>,
    {
        let j = String::deserialize(deserializer)?;
        serde_json::from_str(&j).map_err(de::Error::custom)
    }
}

/// Use a custom ISO 8601 format when serializing and deserializing
/// [`PrimitiveDateTime`][PrimitiveDateTime].
///
/// [PrimitiveDateTime]: ::time::PrimitiveDateTime
pub mod iso8601custom {

    use serde::{ser::Error as _, Deserializer, Serialize, Serializer};
    use time::{
        format_description::well_known::{
            iso8601::{Config, EncodedConfig, TimePrecision},
            Iso8601,
        },
        serde::iso8601,
        PrimitiveDateTime, UtcOffset,
    };

    const FORMAT_CONFIG: EncodedConfig = Config::DEFAULT
        .set_time_precision(TimePrecision::Second {
            decimal_digits: None,
        })
        .encode();

    /// Serialize a [`PrimitiveDateTime`] using the well-known ISO 8601 format.
    pub fn serialize<S>(date_time: &PrimitiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        date_time
            .assume_utc()
            .format(&Iso8601::<FORMAT_CONFIG>)
            .map_err(S::Error::custom)?
            .replace('T', " ")
            .replace('Z', "")
            .serialize(serializer)
    }

    /// Deserialize an [`PrimitiveDateTime`] from its ISO 8601 representation.
    pub fn deserialize<'a, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'a>,
    {
        iso8601::deserialize(deserializer).map(|offset_date_time| {
            let utc_date_time = offset_date_time.to_offset(UtcOffset::UTC);
            PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
        })
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[test]
    fn test_leap_second_parse() {
        #[derive(Serialize, Deserialize)]
        struct Try {
            #[serde(with = "crate::custom_serde::iso8601")]
            f: time::PrimitiveDateTime,
        }
        let leap_second_date_time = json!({"f": "2023-12-31T23:59:60.000Z"});
        let deser = serde_json::from_value::<Try>(leap_second_date_time);

        assert!(deser.is_ok())
    }

    #[test]
    #[allow(clippy::unwrap_used, clippy::indexing_slicing)]
    fn test_iso8601_exact_for_all_millisecond_values() {
        #[derive(Serialize, Deserialize)]
        struct Try {
            #[serde(with = "crate::custom_serde::iso8601")]
            f: time::PrimitiveDateTime,
        }

        let date = time::macros::date!(2026 - 06 - 18);
        for milli in 0..1000u16 {
            let time = time::Time::from_hms_milli(0, 59, 32, milli).unwrap();
            let original = time::PrimitiveDateTime::new(date, time);

            let serialized = serde_json::to_value(&Try { f: original }).unwrap();
            let serialized_str = serialized["f"].as_str().unwrap();
            assert!(
                serialized_str.ends_with(&format!("{milli:03}Z")),
                "millisecond {milli} serialized incorrectly as {serialized_str}"
            );

            let deserialized: Try = serde_json::from_value(serialized).unwrap();
            assert_eq!(deserialized.f, original, "round-trip mismatch for {milli}");
        }
    }

    #[test]
    #[allow(clippy::unwrap_used, clippy::indexing_slicing)]
    fn test_iso8601_micros_exact_for_all_microsecond_values() {
        #[derive(Serialize, Deserialize)]
        struct Try {
            #[serde(with = "crate::custom_serde::iso8601_micros")]
            f: time::PrimitiveDateTime,
        }

        let date = time::macros::date!(2026 - 06 - 18);
        for micro in 0..1_000_000u32 {
            let time = time::Time::from_hms_micro(0, 59, 32, micro).unwrap();
            let original = time::PrimitiveDateTime::new(date, time);

            let serialized = serde_json::to_value(&Try { f: original }).unwrap();
            let serialized_str = serialized["f"].as_str().unwrap();
            assert!(
                serialized_str.ends_with(&format!("{micro:06}Z")),
                "microsecond {micro} serialized incorrectly as {serialized_str}"
            );

            let deserialized: Try = serde_json::from_value(serialized).unwrap();
            assert_eq!(deserialized.f, original, "round-trip mismatch for {micro}");
        }
    }
}
/// Deserialize a comma-separated list from a query parameter into `Option<Vec<T>>`, gracefully ignoring empty strings.
///
/// Example: `?status=posted,archived` → `Option<Vec<EntryStatus>>`
pub mod query_parsers {
    pub fn parse_comma_separated<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Debug + std::fmt::Display + std::error::Error,
    {
        let input: Option<&str> = serde::Deserialize::deserialize(deserializer)?;
        match input {
            Some(s) => {
                let parsed: Vec<T> = s
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|item| item.parse::<T>().map_err(serde::de::Error::custom))
                    .collect::<Result<_, _>>()?;

                if parsed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(parsed))
                }
            }
            None => Ok(None),
        }
    }
}
