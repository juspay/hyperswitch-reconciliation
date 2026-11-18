//! Scheduler interfaces and error types

use async_trait::async_trait;
use time::PrimitiveDateTime;

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

#[async_trait]
pub trait SchedulerInterface: Send + Sync {
    async fn create_job(&self, request: JobRequest) -> Result<String, SchedulerError>;
}

#[derive(Debug, Clone)]
pub struct JobRequest {
    pub endpoint: String,
    pub trigger: JobTrigger,
    /// Creating a job again with the same key and endpoint returns the existing job.
    /// Required for `Delayed` jobs.
    pub idempotency_key: Option<String>,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum JobTrigger {
    Immediate,
    Delayed {
        run_at: PrimitiveDateTime,
    },
    Cron {
        /// 5-field cron (`min hour day-of-month month day-of-week`), no seconds field
        expression: String,
        timezone: String,
    },
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SchedulerError {
    #[error("Invalid job request")]
    InvalidRequest,

    #[error("Failed to create job")]
    JobCreationFailed,

    #[error("Scheduler configuration error")]
    ConfigurationError,
}
