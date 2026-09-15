//! FaaS interfaces and error types

use async_trait::async_trait;
use error_stack::Result;

/// FaaS invocation operations
#[async_trait]
pub trait FaasInterface: Send + Sync {
    /// Queues an invocation and returns. The function's result is not observable.
    async fn invoke_event(&self, function_name: &str, payload: Vec<u8>) -> Result<(), FaasError>;
}

/// Errors that can occur during FaaS operations
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FaasError {
    /// The invoke request failed
    #[error("Failed to invoke lambda function")]
    InvocationFailed,

    /// The request succeeded but the invocation was not queued
    #[error("Lambda invocation was not accepted")]
    NotAccepted,

    /// FaaS backend configuration error
    #[error("Lambda configuration error")]
    ConfigurationError,
}
