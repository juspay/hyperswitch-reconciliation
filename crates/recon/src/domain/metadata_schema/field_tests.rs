//! Field-level validation and transformation tests
//!
//! This module contains comprehensive tests for individual field types,
//! including validation rules and transformations.

#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_arguments,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::missing_panics_doc,
    dead_code
)]
// Main field tests
mod amount_field_test;
mod balance_direction_field_test;
mod currency_field_test;
mod effective_at_field_test;
mod order_id_field_test;

// Metadata field type tests
mod balance_direction_test;
mod currency_test;
mod datetime_test;
mod enum_test;
mod majorunit_test;
mod minorunit_test;
mod number_test;
mod string_test;

// Helpers
mod base_schema;
mod helpers;
