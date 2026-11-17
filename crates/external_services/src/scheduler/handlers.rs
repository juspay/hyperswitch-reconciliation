//! Scheduler handler implementation

use std::sync::Arc;

use super::{
    config::SchedulerConfig,
    implementations::{InvokrClient, NoopSchedulerClient},
    interfaces::{JobRequest, SchedulerError, SchedulerInterface},
};

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

#[derive(Clone)]
pub struct SchedulerHandler {
    scheduler: Arc<dyn SchedulerInterface>,
}

impl SchedulerHandler {
    pub fn from_config(config: &SchedulerConfig) -> Result<Self, SchedulerError> {
        config.validate()?;

        let scheduler: Arc<dyn SchedulerInterface> = match config {
            SchedulerConfig::Invokr { invokr } => Arc::new(InvokrClient::new(invokr)),
            SchedulerConfig::Noop => Arc::new(NoopSchedulerClient),
        };

        Ok(Self { scheduler })
    }

    pub async fn create_job(&self, request: JobRequest) -> Result<String, SchedulerError> {
        self.scheduler.create_job(request).await
    }
}
