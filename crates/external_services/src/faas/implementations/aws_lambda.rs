//! AWS Lambda implementation

use std::time::Duration;

use async_trait::async_trait;
use aws_config::{meta::region::RegionProviderChain, Region};
use aws_sdk_lambda::{
    config::{timeout::TimeoutConfig, Builder as LambdaConfigBuilder},
    error::DisplayErrorContext,
    primitives::Blob,
    types::InvocationType,
    Client as LambdaClient,
};
use error_stack::{Result, ResultExt};
use hyper::StatusCode;

use crate::faas::{
    config::AwsLambdaConfig,
    interfaces::{FaasError, FaasInterface},
};

/// Falls back to this when `request_timeout_secs` is unset
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// AWS Lambda invocation client
#[derive(Debug, Clone)]
pub struct AwsLambdaClient {
    client: LambdaClient,
}

impl AwsLambdaClient {
    /// Builds the SDK client once - credential provider chain included
    pub async fn new(config: &AwsLambdaConfig) -> Result<Self, FaasError> {
        config.validate().map_err(|validation_error| {
            error_stack::report!(FaasError::ConfigurationError).attach_printable(validation_error)
        })?;

        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        let timeout = Duration::from_secs(
            config
                .request_timeout_secs
                .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS),
        );
        let lambda_config = LambdaConfigBuilder::from(&aws_config)
            .timeout_config(TimeoutConfig::builder().operation_timeout(timeout).build())
            .build();

        Ok(Self {
            client: LambdaClient::from_conf(lambda_config),
        })
    }
}

#[async_trait]
impl FaasInterface for AwsLambdaClient {
    async fn invoke_event(&self, function_name: &str, payload: Vec<u8>) -> Result<(), FaasError> {
        // `DisplayErrorContext` unwraps the SDK's source chain - without it every
        // failure prints as a bare "service error".
        let output = self
            .client
            .invoke()
            .function_name(function_name)
            .invocation_type(InvocationType::Event)
            .payload(Blob::new(payload))
            .send()
            .await
            .map_err(|err| {
                error_stack::report!(FaasError::InvocationFailed).attach_printable(format!(
                    "Lambda invoke failed: {function_name}: {}",
                    DisplayErrorContext(&err)
                ))
            })?;

        // Event invocations are queued, never run inline - Lambda answers 202 Accepted.
        match output.status_code() {
            code if code == i32::from(StatusCode::ACCEPTED.as_u16()) => Ok(()),
            status => Err(FaasError::NotAccepted)
                .attach_printable_lazy(|| format!("Not queued: {function_name}, status={status}")),
        }
    }
}
