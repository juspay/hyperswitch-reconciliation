//! FaaS configuration

use serde::Deserialize;

/// Configuration for FaaS backends
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "faas_backend")]
#[serde(rename_all = "snake_case")]
pub enum FaasConfig {
    /// Logs and returns success - local and CI default
    #[default]
    Noop,
    /// AWS Lambda
    AwsLambda {
        /// AWS Lambda specific configuration
        aws_lambda: AwsLambdaConfig,
    },
}

/// AWS Lambda configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AwsLambdaConfig {
    /// AWS region the functions live in
    pub region: String,
    /// Per-invocation request timeout, defaults to 30s
    pub request_timeout_secs: Option<u64>,
}

impl AwsLambdaConfig {
    /// Validates the AWS Lambda configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.region.trim().is_empty() {
            return Err("AWS Lambda region cannot be empty".to_string());
        }
        Ok(())
    }
}

impl FaasConfig {
    /// Validates the FaaS configuration
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::AwsLambda { aws_lambda } => aws_lambda.validate(),
            Self::Noop => Ok(()),
        }
    }
}
