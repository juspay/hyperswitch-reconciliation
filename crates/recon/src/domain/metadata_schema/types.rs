//! Core types for metadata validation pipeline
//!
//! This module contains all the data structures used in the validation pipeline:
//! - ValidationRequest/Response for engine interface
//! - MappedEntryData for mapper output
//! - TransformationStagingEntryData for validated output
//! - Error types with UUID tracking

use api_models::transformation::{
    RowError as RowErrorApi, ValidatedEntryData as ValidatedEntryDataApi,
};
use common_enums::{BalanceDirection, Currency};
use common_types::{
    metadata_schema::RuleType,
    recon_rule::entry_field::EntryField,
    units::{FloatMajorUnit, MinorUnit},
};
use common_utils::errors::ErrorSwitchFrom;
use error_stack::Report;
use std::collections::HashMap;
use thiserror::Error;
use time::PrimitiveDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    domain::transformation::types::{LocatedValue, SourceLocation, SourceRecord},
    errors::metadata_schema::MetadataSchemaError,
};

#[derive(Debug, Clone)]
pub struct ParsedRow {
    pub source: SourceRecord,
    pub fields: HashMap<String, LocatedValue<Option<String>>>,
}

impl ParsedRow {
    pub fn from_api_fields(fields: HashMap<String, Option<String>>, index: Option<usize>) -> Self {
        Self {
            source: SourceRecord::ApiRecord { index },
            fields: fields
                .into_iter()
                .map(|(field_name, value)| {
                    let location = SourceLocation::Field {
                        field_name: field_name.clone(),
                    };
                    (field_name, LocatedValue { value, location })
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MappedRow {
    pub source: SourceRecord,
    pub mapped_fields: HashMap<String, LocatedValue<Option<String>>>,
}

impl MappedRow {
    pub fn from_api_fields(
        mapped_fields: HashMap<String, Option<String>>,
        index: Option<usize>,
    ) -> Self {
        Self {
            source: SourceRecord::ApiRecord { index },
            mapped_fields: mapped_fields
                .into_iter()
                .map(|(field_name, value)| {
                    let location = SourceLocation::Field {
                        field_name: field_name.clone(),
                    };
                    (field_name, LocatedValue { value, location })
                })
                .collect(),
        }
    }
}

// ===== Validation Types =====

#[derive(Debug, Clone)]
pub enum RowValidationOutcome {
    Validated(ValidatedEntryData),
    Rejected(RowRejection),
}

#[derive(Debug, Clone)]
pub enum FieldValidationOutcome<T> {
    Valid(T),
    Invalid(LocatedValue<FieldError>),
}

impl<T> FieldValidationOutcome<T> {
    pub fn invalid(error: FieldError, location: &SourceLocation) -> Self {
        Self::Invalid(LocatedValue {
            value: error,
            location: location.clone(),
        })
    }

    pub fn map<U>(self, transform: impl FnOnce(T) -> U) -> FieldValidationOutcome<U> {
        match self {
            Self::Valid(value) => FieldValidationOutcome::Valid(transform(value)),
            Self::Invalid(error) => FieldValidationOutcome::Invalid(error),
        }
    }

    pub fn and_then<U>(
        self,
        next: impl FnOnce(T) -> FieldValidationOutcome<U>,
    ) -> FieldValidationOutcome<U> {
        match self {
            Self::Valid(value) => next(value),
            Self::Invalid(error) => FieldValidationOutcome::Invalid(error),
        }
    }

    pub fn from_required_value(
        value: Result<Option<T>, Report<ValueError>>,
        identifier: &str,
        location: &SourceLocation,
    ) -> Self {
        match value {
            Ok(Some(value)) => Self::Valid(value),
            Ok(None) => Self::invalid(
                FieldError::RequiredFieldMissing(identifier.to_string()),
                location,
            ),
            Err(error) => Self::invalid(
                error
                    .current_context()
                    .to_field_error(identifier.to_string()),
                location,
            ),
        }
    }

    pub fn from_required_value_located(
        value: Result<Option<T>, Report<ValueError>>,
        identifier: &str,
        location: &SourceLocation,
    ) -> FieldValidationOutcome<LocatedValue<T>> {
        Self::from_required_value(value, identifier, location).map(|value| LocatedValue {
            value,
            location: location.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RowRejection {
    pub source_record: SourceRecord,
    pub field_errors: Vec<LocatedValue<FieldError>>,
}

impl RowRejection {
    pub fn to_api_response(&self) -> RowErrorApi {
        RowErrorApi {
            errors: self
                .field_errors
                .iter()
                .map(|located| (located.value.get_identifier(), located.value.get_error()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FileParsingOutcome {
    Parsed { rows: Vec<RowParsingOutcome> },
    Rejected { failure: ParsingFailure },
}

impl FileParsingOutcome {
    pub fn rejected(failure: ParsingFailure) -> Self {
        Self::Rejected { failure }
    }
}

#[derive(Debug, Clone)]
pub enum RowParsingOutcome {
    Parsed(ParsedRow),
    Rejected(ParsingDiagnostic),
}

#[derive(Debug, Clone)]
pub struct ParsingDiagnostic {
    pub location: SourceLocation,
    pub message: String,
}

#[derive(Debug, Clone, Error)]
pub enum ParsingFailure {
    #[error("{details}")]
    InvalidFormat { details: String },
    #[error("Header row not found")]
    HeaderNotFound,
    #[error("Configured sheet was not found")]
    SheetNotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(tag = "type", content = "value")]
pub enum Amount {
    MinorUnit(MinorUnit),
    FloatMajorUnit(FloatMajorUnit),
}

impl Amount {
    pub fn to_minor_unit(&self, currency: Currency) -> Result<MinorUnit, String> {
        match self {
            Self::MinorUnit(minor_unit) => Ok(*minor_unit),
            Self::FloatMajorUnit(float_major_unit) => float_major_unit
                .to_minor_unit_as_i64(currency)
                .map_err(|e| format!("Failed to convert FloatMajorUnit to MinorUnit: {:?}", e)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValidatedFieldValue {
    Amount(Amount),
    Currency(Currency),
    BalanceDirection(BalanceDirection),
    DateTime(PrimitiveDateTime),
    String(String),
    Number(f64),
    Enum(String),
}

impl ValidatedFieldValue {
    pub fn to_json(&self) -> Result<serde_json::Value, ValueError> {
        match self {
            Self::Amount(amount) => match amount {
                Amount::MinorUnit(minor_unit) => {
                    Ok(serde_json::Value::String(minor_unit.to_string()))
                }
                Amount::FloatMajorUnit(float_major_unit) => {
                    Ok(serde_json::Value::String(float_major_unit.to_string()))
                }
            },
            Self::Currency(currency) => Ok(serde_json::Value::String(currency.to_string())),
            Self::BalanceDirection(direction) => {
                Ok(serde_json::Value::String(direction.to_string()))
            }
            Self::DateTime(datetime) => Ok(serde_json::Value::String(datetime.to_string())),
            Self::String(s) => Ok(serde_json::Value::String(s.clone())),
            Self::Number(n) => serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .ok_or(ValueError::JsonParseError {
                    actual_value: n.to_string(),
                }),
            Self::Enum(s) => Ok(serde_json::Value::String(s.clone())),
        }
    }
}

/// Unified staging entry data structure with UUID-based identification
/// This replaces the fragile index-based mapping approach
#[derive(Debug, Clone)]
pub struct ValidatedEntryData {
    pub uuid: Uuid,
    pub source_record: SourceRecord,
    pub row_data: EntryData,
}

/// Complete row data for staging entries - same structure for all transformation types
#[derive(Debug, Clone)]
pub struct EntryData {
    pub balance_direction: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub effective_at: PrimitiveDateTime,
    pub order_id: String,
    pub metadata: serde_json::Value,
    pub exposed_metadata: serde_json::Value,
    pub searchable_metadata: serde_json::Value,
}

impl ValidatedEntryData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uuid: Uuid,
        source_record: SourceRecord,
        balance_direction: BalanceDirection,
        amount: MinorUnit,
        currency: Currency,
        effective_at: PrimitiveDateTime,
        order_id: String,
        metadata: serde_json::Value,
        exposed_metadata: serde_json::Value,
        searchable_metadata: serde_json::Value,
    ) -> Self {
        Self {
            uuid,
            source_record,
            row_data: EntryData {
                balance_direction,
                amount,
                currency,
                effective_at,
                order_id,
                metadata,
                exposed_metadata,
                searchable_metadata,
            },
        }
    }

    pub fn to_api_response(&self) -> ValidatedEntryDataApi {
        self.row_data.to_api_response()
    }
}

impl EntryData {
    pub fn to_api_response(&self) -> ValidatedEntryDataApi {
        ValidatedEntryDataApi {
            entry_type: self.balance_direction,
            amount: self.amount,
            currency: self.currency,
            effective_at: self.effective_at,
            order_id: self.order_id.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

/// Detailed validation error for a specific field
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum FieldError {
    #[error("Missing Required Field: {0}")]
    RequiredFieldMissing(String),
    #[error("Field not Mapped: {0}")]
    FieldNotMapped(String),
    #[error("Value error in field '{identifier}': {error}")]
    ValueError {
        identifier: String,
        #[source]
        error: ValueError,
    },
}

impl FieldError {
    pub fn from_value_error(report_err: Report<ValueError>, identifier: String) -> Report<Self> {
        let value_err = report_err.current_context().clone();

        report_err.change_context(value_err.to_field_error(identifier))
    }

    pub fn get_identifier(&self) -> String {
        match self {
            Self::RequiredFieldMissing(field) => field.clone(),
            Self::FieldNotMapped(field) => field.clone(),
            Self::ValueError { identifier, .. } => identifier.clone(),
        }
    }

    pub fn get_error(&self) -> String {
        match self {
            Self::RequiredFieldMissing(_) => "Missing Required Field".to_string(),
            Self::FieldNotMapped(_) => "Field Not Mapped".to_string(),
            Self::ValueError { error, .. } => error.to_string(),
        }
    }
}

/// Value parsing and format validation errors
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ValueError {
    #[error("Invalid Type: {actual_value}")]
    InvalidType { actual_value: String },
    #[error("Invalid Format: {actual_value:?}")]
    InvalidFormat { actual_value: Option<String> },
    #[error("Invalid Balance Direction: {actual_value}")]
    InvalidBalanceDirection { actual_value: String },
    #[error("Invalid Currency: {actual_value:?} ({parse_error})")]
    InvalidCurrency {
        actual_value: Option<String>,
        parse_error: String,
    },
    #[error("Invalid DateTime: {actual_value}")]
    InvalidDateTime { actual_value: String },
    #[error("Invalid Enum Value: {actual_value:?}. Allowed values: {allowed_values:?}")]
    InvalidEnumValue {
        actual_value: String,
        allowed_values: Vec<String>,
    },
    #[error("Failed to Parse value to json")]
    JsonParseError { actual_value: String },
    #[error("Rule validation failed for value '{actual_value:?}': {rule_error}")]
    RuleError {
        actual_value: Option<String>,
        #[source]
        rule_error: RuleError,
    },
}

impl ValueError {
    pub fn to_field_error(&self, identifier: String) -> FieldError {
        FieldError::ValueError {
            identifier,
            error: self.clone(),
        }
    }

    pub fn from_rule_error(report_err: Report<RuleError>, value: Option<String>) -> Report<Self> {
        let rule_error = report_err.current_context().clone();

        report_err.change_context(rule_error.to_value_error(value))
    }
}

/// Rule validation errors (constraints, ranges, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum RuleError {
    #[error("Value Too Long (max: {max_length})")]
    ValueTooLong { max_length: String },
    #[error("Value Too Short (min: {min_length})")]
    ValueTooShort { min_length: String },
    #[error("Value Too Small (min: {min_value})")]
    ValueTooSmall { min_value: String },
    #[error("Value Too Large (max: {max_value})")]
    ValueTooLarge { max_value: String },
    #[error("Value is Negative")]
    NegativeValue,
    #[error("Prefix Not Found: {prefix}")]
    PrefixNotFound { prefix: String },
    #[error("Suffix Not Found: {suffix}")]
    SuffixNotFound { suffix: String },
    #[error("Failed to parse value as JSON")]
    JsonParseFailed,
    #[error("JSON key not found: {key}")]
    JsonKeyNotFound { key: String },
    #[error("Regex did not match any part of the value (pattern: {pattern})")]
    UnmatchedRegex { pattern: String },
    #[error("Invalid config error: {details}")]
    InvalidConfig { details: String },
    #[error("Invalid DateTime transformation: {details}")]
    InvalidDateTimeConfig { details: String },
}

impl RuleError {
    pub fn to_value_error(&self, actual_value: Option<String>) -> ValueError {
        ValueError::RuleError {
            actual_value,
            rule_error: self.clone(),
        }
    }
}

pub struct RuleConfig {
    pub rule_type: RuleType,
    pub value: String,
    pub detail: String,
}

/// Configuration validation errors for metadata schemas
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ConfigValidationError {
    // Identifier validation
    #[error("Empty field identifier for field name: {field_name}")]
    EmptyFieldIdentifier { field_name: EntryField },

    #[error("Empty string value")]
    EmptyString,

    #[error("Empty field name for identifier: {identifier}")]
    EmptyFieldName { identifier: String },

    #[error("Duplicate metadata field name: '{field_name}'")]
    DuplicateMetadataFieldName { field_name: EntryField },

    // Field description validation
    #[error("Empty field description for identifier: '{identifier}'")]
    EmptyFieldDescription { identifier: String },

    // Unique constraints validation
    #[error("Unique constraint references non-existent field: '{field_name}'")]
    UniqueConstraintInvalidFieldReference { field_name: EntryField },

    #[error("Composite unique constraint must contain at least one field")]
    EmptyCompositeUniqueConstraint,

    #[error("Invalid number value: '{value}'")]
    InvalidNumber { value: String },

    #[error("identifier: {identifier}, field_name: {field_name}, Field error: {error}")]
    FieldError {
        #[source]
        error: ConfigFieldValidationError,
        identifier: String,
        field_name: EntryField,
    },

    #[error("Invalid date time config")]
    InvalidDateTimeConfig,
}

impl ErrorSwitchFrom<ConfigValidationError> for MetadataSchemaError {
    fn switch_from(error: &ConfigValidationError) -> Self {
        Self::MetadataConfigError(error.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ConfigFieldValidationError {
    #[error("Multiple rules of same type not allowed: {rule_type:?}")]
    MultipleRulesOfSameTypeNotAllowed { rule_type: RuleType },

    // Rule syntax validation
    #[error("Invalid rule syntax for {rule_type:?}: {details}")]
    InvalidRuleSyntax {
        rule_type: RuleType,
        details: String,
    },

    // Balance direction validation
    #[error("Balance direction arrays cannot be empty")]
    EmptyBalanceDirectionValues,

    #[error("Duplicate balance direction value: '{value}'")]
    DuplicateBalanceDirectionValue { value: String },

    #[error("Empty balance direction value not allowed")]
    EmptyBalanceDirectionValue,

    #[error("Invalid date time config")]
    InvalidDateTimeConfig,

    #[error("Enum mappings cannot be empty")]
    EmptyEnumMappings,

    #[error("Duplicate enum key: '{key}'")]
    DuplicateEnumKey { key: String },

    #[error("Empty enum key not allowed")]
    EmptyEnumKey,

    #[error("Empty enum value for key: '{key}'")]
    EmptyEnumValue { key: String },

    #[error("Invalid regex pattern: {pattern} ({reason})")]
    RegexCompileFailed { pattern: String, reason: String },

    #[error("Regex group {group} is out of range for pattern `{pattern}` (pattern has {capture_count} capture group(s))")]
    RegexGroupOutOfRange {
        pattern: String,
        group: usize,
        capture_count: usize,
    },

    #[error("Prefix for StartsWith transformation rule cannot be empty")]
    EmptyStartsWithRulePrefix,
}
