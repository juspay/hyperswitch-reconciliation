//! Scheduler handler implementation

use std::sync::Arc;

use error_stack::Result;

use super::{
    config::SchedulerConfig,
    implementations::{InvokrClient, NoopSchedulerClient},
    interfaces::{JobRequest, SchedulerError, SchedulerInterface},
};

/// Handler for scheduling
#[derive(Clone)]
pub struct SchedulerHandler {
    scheduler: Arc<dyn SchedulerInterface>,
}

impl SchedulerHandler {
    /// Creates a scheduler handler from configuration
    pub fn from_config(config: &SchedulerConfig) -> Result<Self, SchedulerError> {
        config.validate()?;

        let scheduler: Arc<dyn SchedulerInterface> = match config {
            SchedulerConfig::Invokr { invokr } => Arc::new(InvokrClient::new(invokr)),
            SchedulerConfig::Noop => Arc::new(NoopSchedulerClient),
        };

        Ok(Self { scheduler })
    }

    /// Registers a job on the configured backend and returns its job id
    pub async fn create_job(&self, request: JobRequest) -> Result<String, SchedulerError> {
        self.scheduler.create_job(request).await
    }
}
