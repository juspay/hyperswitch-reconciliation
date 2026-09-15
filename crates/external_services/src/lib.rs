//! External services integration crate
//!
//! This crate provides integrations with external services including:
//! - Secret management (AWS KMS, No Encryption)
//! - File storage (AWS S3, Local File System)
//! - FaaS invocation (AWS Lambda, Noop)
//! - Application encryption (Local, Keymanager Service)
//! - HyperSwitch client
//! - Future: Other external service integrations

pub mod encryption;
pub mod faas;
pub mod file_storage;
pub mod http_client;
pub mod hyperswitch;
pub mod secrets;
pub mod user;

// Re-export the main interfaces for convenience
pub use encryption::{ApplicationEncryptionClient, ApplicationEncryptionConfig};
pub use faas::{FaasConfig, FaasError, FaasHandler, FaasInterface};
pub use file_storage::{
    FileStorageConfig, FileStorageError, FileStorageHandler, FileStorageInterface,
};
pub use hyperswitch::HyperswitchConfig;
pub use user::{UserClient, UserConfig, UserResponse};
