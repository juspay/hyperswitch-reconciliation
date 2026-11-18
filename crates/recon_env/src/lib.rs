//! Environment detection, logging, request ID middleware and config loading for the recon crates.
//!
//! This crate must not depend on `common_utils`: `common_utils` depends on it for logging.

pub mod config;
pub mod env;
pub mod logging;
pub mod request_id;
