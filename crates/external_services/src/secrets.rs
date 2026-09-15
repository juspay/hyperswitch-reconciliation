//! Secret management module
//!
//! This module provides interfaces and implementations for managing application secrets
//! with support for various backends including AWS KMS and no encryption.

pub mod config;
pub mod implementations;

// Re-export main types for convenience
pub use config::SecretsManagementConfig;
