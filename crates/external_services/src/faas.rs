//! FaaS invocation (AWS Lambda, Noop)

pub mod config;
pub mod handlers;
pub mod implementations;
pub mod interfaces;

pub use config::FaasConfig;
pub use handlers::FaasHandler;
pub use interfaces::{FaasError, FaasInterface};
