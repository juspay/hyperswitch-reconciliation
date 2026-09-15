//! HyperSwitch client module
//!
//! This module provides integration with HyperSwitch for user management operations.
//!
//! Two client modes are available:
//! - `LocalService`: Returns mock data (for testing and local development)
//! - `HyperswitchService`: Makes actual HTTP calls to HyperSwitch

pub mod client;
pub mod config;
pub mod implementations;
pub mod interfaces;
pub mod types;

// Re-export main types for convenience
pub use client::user::UserClient;
pub use config::user::UserConfig;
pub use types::response::UserResponse;
