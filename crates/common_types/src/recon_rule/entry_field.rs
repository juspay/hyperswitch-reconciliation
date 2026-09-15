use crate::units::MinorUnit;
use common_enums::BalanceDirection;
use common_utils::crypto::EncryptableValue;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Text};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};
use thiserror::Error;
use time::PrimitiveDateTime;
use utoipa::ToSchema;

/// Represents a field in a staging entry or transaction entry
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Hash, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = Text)]
pub enum EntryField {
    Amount,
    Currency,
    EntryType,
    EffectiveAt,
    OrderId,
    Metadata(String),
}

impl EntryField {
    pub fn is_metadata(&self) -> bool {
        matches!(self, Self::Metadata(_))
    }
}

/// Display implementation for EntryField - converts to string format
/// This is the single source of truth for field to string conversion
impl fmt::Display for EntryField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Amount => write!(f, "amount"),
            Self::Currency => write!(f, "currency"),
            Self::EntryType => write!(f, "entry_type"),
            Self::EffectiveAt => write!(f, "effective_at"),
            Self::OrderId => write!(f, "order_id"),
            Self::Metadata(key) => write!(f, "metadata.{}", key),
        }
    }
}

/// FromStr implementation for EntryField - parses from string format
/// This is the single source of truth for string to field conversion
impl FromStr for EntryField {
    type Err = ParseEntryFieldError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amount" => Ok(Self::Amount),
            "currency" => Ok(Self::Currency),
            "entry_type" => Ok(Self::EntryType),
            "effective_at" => Ok(Self::EffectiveAt),
            "order_id" => Ok(Self::OrderId),
            _ => {
                if let Some(key) = s.strip_prefix("metadata.") {
                    Ok(Self::Metadata(key.to_string()))
                } else {
                    Err(ParseEntryFieldError {
                        invalid_field: s.to_string(),
                    })
                }
            }
        }
    }
}

/// Manual Serialize implementation - delegates to Display
impl Serialize for EntryField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Manual Deserialize implementation - delegates to FromStr
impl<'de> Deserialize<'de> for EntryField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

common_utils::impl_to_sql_from_sql_text!(EntryField);

/// Error type for parsing EntryField from string
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid entry field: {invalid_field}")]
pub struct ParseEntryFieldError {
    pub invalid_field: String,
}

/// Trait for types that can extract field values based on EntryField enum
pub trait ExtractFromEntryField: ExtractAccountId {
    /// Extract a field value as a string
    fn extract_field(&self, field_name: &EntryField) -> Option<String>;

    fn currency(&self) -> common_enums::Currency;

    fn entry_type(&self) -> BalanceDirection;

    fn amount(&self) -> MinorUnit;

    fn metadata(&self) -> &EncryptableValue;

    fn effective_at(&self) -> PrimitiveDateTime;

    fn order_id(&self) -> String;
}

pub trait ExtractAccountId {
    fn extract_account_id(&self) -> String;
}

impl EntryField {
    pub fn to_field_path_without_metadata_prefix(&self) -> String {
        match self {
            Self::Amount => "amount".into(),
            Self::Currency => "currency".into(),
            Self::EntryType => "entry_type".into(),
            Self::EffectiveAt => "effective_at".into(),
            Self::OrderId => "order_id".into(),
            Self::Metadata(key) => key.to_string(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(EntryField::Amount, "\"amount\"")]
    #[case(EntryField::Currency, "\"currency\"")]
    #[case(EntryField::EntryType, "\"entry_type\"")]
    #[case(EntryField::EffectiveAt, "\"effective_at\"")]
    #[case(EntryField::OrderId, "\"order_id\"")]
    #[case(EntryField::Metadata("transaction_id".to_string()), "\"metadata.transaction_id\"")]
    #[case(EntryField::Metadata("custom_key".to_string()), "\"metadata.custom_key\"")]
    fn test_serialize_entry_field(#[case] field: EntryField, #[case] expected_json: &str) {
        assert_eq!(serde_json::to_string(&field).unwrap(), expected_json);
    }

    #[rstest]
    #[case("\"amount\"", EntryField::Amount)]
    #[case("\"currency\"", EntryField::Currency)]
    #[case("\"entry_type\"", EntryField::EntryType)]
    #[case("\"effective_at\"", EntryField::EffectiveAt)]
    #[case("\"order_id\"", EntryField::OrderId)]
    #[case("\"metadata.order_id\"", EntryField::Metadata("order_id".to_string()))]
    #[case("\"metadata.transaction_id\"", EntryField::Metadata("transaction_id".to_string()))]
    fn test_deserialize_entry_field(#[case] json: &str, #[case] expected_field: EntryField) {
        let parsed: EntryField = serde_json::from_str(json).unwrap();
        assert_eq!(parsed, expected_field);
    }

    #[rstest]
    #[case(EntryField::Amount)]
    #[case(EntryField::Currency)]
    #[case(EntryField::EntryType)]
    #[case(EntryField::EffectiveAt)]
    #[case(EntryField::OrderId)]
    #[case(EntryField::Metadata("test_key".to_string()))]
    fn test_round_trip_serialization(#[case] field: EntryField) {
        let json = serde_json::to_string(&field).unwrap();
        let parsed: EntryField = serde_json::from_str(&json).unwrap();
        assert_eq!(field, parsed);
    }

    #[rstest]
    #[case("\"invalid_field_name\"")]
    #[case("\"meta\"")]
    #[case("\"metadata\"")]
    #[case("\"random\"")]
    fn test_invalid_field_name_error(#[case] invalid_json: &str) {
        let result = serde_json::from_str::<EntryField>(invalid_json);
        assert!(result.is_err());
    }

    // Display trait tests
    #[rstest]
    #[case(EntryField::Amount, "amount")]
    #[case(EntryField::Currency, "currency")]
    #[case(EntryField::EntryType, "entry_type")]
    #[case(EntryField::EffectiveAt, "effective_at")]
    #[case(EntryField::OrderId, "order_id")]
    #[case(EntryField::Metadata("transaction_id".to_string()), "metadata.transaction_id")]
    #[case(EntryField::Metadata("custom_key".to_string()), "metadata.custom_key")]
    fn test_display_entry_field(#[case] field: EntryField, #[case] expected_string: &str) {
        assert_eq!(field.to_string(), expected_string);
    }

    // FromStr trait tests
    #[rstest]
    #[case("amount", EntryField::Amount)]
    #[case("currency", EntryField::Currency)]
    #[case("entry_type", EntryField::EntryType)]
    #[case("effective_at", EntryField::EffectiveAt)]
    #[case("order_id", EntryField::OrderId)]
    #[case("metadata.order_id", EntryField::Metadata("order_id".to_string()))]
    #[case("metadata.transaction_id", EntryField::Metadata("transaction_id".to_string()))]
    fn test_from_str_entry_field(#[case] input: &str, #[case] expected_field: EntryField) {
        let parsed: EntryField = input.parse().unwrap();
        assert_eq!(parsed, expected_field);
    }

    #[rstest]
    #[case("invalid_field_name")]
    #[case("meta")]
    #[case("metadata")]
    #[case("random")]
    fn test_from_str_invalid_field_name(#[case] invalid_input: &str) {
        let result = invalid_input.parse::<EntryField>();
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.invalid_field, invalid_input);
        }
    }

    // Round-trip tests for Display and FromStr
    #[rstest]
    #[case(EntryField::Amount)]
    #[case(EntryField::Currency)]
    #[case(EntryField::EntryType)]
    #[case(EntryField::EffectiveAt)]
    #[case(EntryField::OrderId)]
    #[case(EntryField::Metadata("test_key".to_string()))]
    fn test_display_from_str_round_trip(#[case] field: EntryField) {
        let string = field.to_string();
        let parsed: EntryField = string.parse().unwrap();
        assert_eq!(field, parsed);
    }
}
