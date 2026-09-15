//! Metadata Schema Validation Domain
//!
//! This module provides the core business logic for validating data against metadata schemas.
//! It includes:
//! - Data mapping from raw fields to internal structures
//! - Schema-driven field validation
//! - UUID-based tracking throughout the validation pipeline
//!
//! The validation pipeline follows: Raw Data (File) → Parse → Map → Validate → Validated Data

pub mod config_validator;
pub mod data_validator;
pub mod implementations;
pub mod mapper;
pub mod skip_config;
pub mod types;

// Individual field validator modules
pub mod fields {
    pub mod amount_field;
    pub mod balance_direction_field;
    pub mod currency_field;
    pub mod effective_at_field;
    pub mod order_id_field;
}

// Datatype validation implementations
pub mod datatypes {
    pub mod balance_direction;
    pub mod currency;
    pub mod datetime;
    pub mod enum_type;
    pub mod majorunit;
    pub mod minorunit;
    pub mod number;
    pub mod string;
}

// Shared transformation helpers used across datatypes
pub mod transformations {
    pub mod datetime_post_parse;
    pub mod json_extract;
    pub mod regex_extract;
    pub mod replace_char;
}

// Re-export main components for easy access
pub use config_validator::ConfigValidator;
pub use data_validator::RowValidator;
pub use mapper::{MetadataSchemaMapper, RowMapper};
pub use types::{
    FieldError, MappedRow, ParsedRow, RowRejection, RowValidationOutcome, ValidatedEntryData,
    ValueError,
};

// Test modules
#[cfg(test)]
mod field_tests;
