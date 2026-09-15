//! Application encryption module
//!
//! This module provides interfaces and implementations for application data encryption
//! with support for local encryption using master keys and keymanager service.

pub mod config;
pub mod implementations;

// Re-export main types for convenience
pub use config::{ApplicationEncryptionClient, ApplicationEncryptionConfig};
