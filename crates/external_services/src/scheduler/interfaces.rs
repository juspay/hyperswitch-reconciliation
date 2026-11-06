//! Scheduler interfaces and error types

use async_trait::async_trait;
use error_stack::Result;
use time::PrimitiveDateTime;

/// Scheduling operations
#[async_trait]
pub trait SchedulerInterface: Send + Sync {
    /// Registers a job and returns the scheduler's job id, needed to cancel or look up
    /// the job later. The scheduler later calls the job's endpoint with `input`;
    /// whether that call succeeds is not observable here.
    async fn create_job(&self, request: JobRequest) -> Result<String, SchedulerError>;
}

/// A job to hand to the scheduler
#[derive(Debug, Clone)]
pub struct JobRequest {
    /// Name of an endpoint already registered with the scheduler
    pub endpoint: String,
    /// When the job fires
    pub trigger: JobTrigger,
    /// Creating a job again with the same key and endpoint returns the existing job.
    /// Required for `Delayed` jobs.
    pub idempotency_key: Option<String>,
    /// Body sent to the endpoint; must be a JSON object
    pub input: Option<serde_json::Value>,
}

/// When a job fires
#[derive(Debug, Clone)]
pub enum JobTrigger {
    /// As soon as a worker picks it up
    Immediate,
    /// Once, at `run_at` (UTC)
    Delayed { run_at: PrimitiveDateTime },
    /// Repeatedly, on a cron schedule
    Cron {
        /// 5-field cron (`min hour day-of-month month day-of-week`), no seconds field
        expression: String,
        /// IANA timezone name, e.g. `Asia/Kolkata`. Required by Invokr
        timezone: String,
    },
}

/// Errors that can occur during scheduler operations
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SchedulerError {
    /// The request was invalid before it was sent
    #[error("Invalid job request")]
    InvalidRequest,

    /// The create-job request failed
    #[error("Failed to create job")]
    JobCreationFailed,

    /// Scheduler backend configuration error
    #[error("Scheduler configuration error")]
    ConfigurationError,
}
