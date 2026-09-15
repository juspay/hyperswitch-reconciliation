//! FaaS implementations

pub mod aws_lambda;
pub mod noop;

pub use aws_lambda::AwsLambdaClient;
pub use noop::NoopFaasClient;
