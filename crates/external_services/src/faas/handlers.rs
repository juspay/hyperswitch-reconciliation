//! FaaS handler implementation

use std::sync::Arc;

use super::{
    config::FaasConfig,
    implementations::{AwsLambdaClient, NoopFaasClient},
    interfaces::{FaasError, FaasInterface},
};

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

/// Handler for FaaS invocations
#[derive(Clone)]
pub struct FaasHandler {
    faas: Arc<dyn FaasInterface>,
}

impl FaasHandler {
    /// Creates a FaaS handler from configuration
    pub async fn from_config(config: &FaasConfig) -> Result<Self, FaasError> {
        config.validate().map_err(|validation_error| {
            error_stack::Report::new(FaasError::ConfigurationError).attach(validation_error)
        })?;

        let faas: Arc<dyn FaasInterface> = match config {
            FaasConfig::AwsLambda { aws_lambda } => {
                Arc::new(AwsLambdaClient::new(aws_lambda).await?)
            }
            FaasConfig::Noop => Arc::new(NoopFaasClient),
        };

        Ok(Self { faas })
    }

    /// Queues an invocation on the configured backend
    pub async fn invoke_event(
        &self,
        function_name: &str,
        payload: Vec<u8>,
    ) -> Result<(), FaasError> {
        self.faas.invoke_event(function_name, payload).await
    }
}
