//! File storage module
//!
//! This module provides interfaces and implementations for file storage operations
//! with support for various backends including AWS S3 and local filesystem.

pub mod config;
pub mod handlers;
pub mod implementations;
pub mod interfaces;

// Re-export main types for convenience
pub use config::FileStorageConfig;
pub use handlers::FileStorageHandler;
pub use interfaces::{FileStorageError, FileStorageInterface};
