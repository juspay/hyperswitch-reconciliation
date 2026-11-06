//! No-op scheduler implementation - local and CI, where no Invokr is running

use async_trait::async_trait;
use error_stack::Result;

use crate::scheduler::interfaces::{JobRequest, SchedulerError, SchedulerInterface};

/// Job id returned by the noop backend
pub const NOOP_JOB_ID: &str = "noop_job";

/// No-op scheduler client
#[derive(Debug, Clone, Default)]
pub struct NoopSchedulerClient;

#[async_trait]
impl SchedulerInterface for NoopSchedulerClient {
    /// Returns a fixed placeholder id, since no job exists
    async fn create_job(&self, request: JobRequest) -> Result<String, SchedulerError> {
        tracing::info!(
            endpoint = request.endpoint,
            trigger = ?request.trigger,
            "Noop scheduler backend: skipping job creation"
        );
        Ok(NOOP_JOB_ID.to_string())
    }
}
