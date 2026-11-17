//! No-op scheduler for local and CI, where no Invokr is running

use async_trait::async_trait;

use crate::scheduler::interfaces::{JobRequest, SchedulerError, SchedulerInterface};

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

pub const NOOP_JOB_ID: &str = "noop_job";

#[derive(Debug, Clone, Default)]
pub struct NoopSchedulerClient;

#[async_trait]
impl SchedulerInterface for NoopSchedulerClient {
    async fn create_job(&self, request: JobRequest) -> Result<String, SchedulerError> {
        tracing::info!(
            endpoint = request.endpoint,
            trigger = ?request.trigger,
            "Noop scheduler backend: skipping job creation"
        );
        Ok(NOOP_JOB_ID.to_string())
    }
}
