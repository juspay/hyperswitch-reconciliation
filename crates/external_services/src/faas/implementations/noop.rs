//! No-op FaaS implementation - local and CI, where no AWS credentials exist

use async_trait::async_trait;

use crate::faas::interfaces::{FaasError, FaasInterface};

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

/// No-op FaaS client
#[derive(Debug, Clone, Default)]
pub struct NoopFaasClient;

#[async_trait]
impl FaasInterface for NoopFaasClient {
    async fn invoke_event(&self, function_name: &str, payload: Vec<u8>) -> Result<(), FaasError> {
        tracing::info!(
            function_name,
            payload_bytes = payload.len(),
            "Noop FaaS backend: skipping invocation"
        );
        Ok(())
    }
}
