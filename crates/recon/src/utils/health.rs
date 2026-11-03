use std::{
    future::Future,
    time::{Duration, Instant},
};

use api_models::health::ComponentHealth;
use common_utils::logging;
use tokio::sync::watch;

const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum HealthComponent {
    Database,
    Encryption,
    UserClient,
}

pub async fn run_check<C: error_stack::Context>(
    component: HealthComponent,
    check: impl Future<Output = error_stack::Result<(), C>>,
) -> ComponentHealth {
    let start = Instant::now();
    let result = tokio::time::timeout(CHECK_TIMEOUT, check).await;
    let latency_ms = Some(u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX));

    let error = match result {
        Ok(Ok(())) => return ComponentHealth::Healthy { latency_ms },
        Ok(Err(report)) => {
            logging::error!(component = %component, error = ?report, "Deep health check failed");
            report.current_context().to_string()
        }
        Err(_) => {
            logging::error!(component = %component, "Deep health check timed out");
            format!("Timed out after {}s", CHECK_TIMEOUT.as_secs())
        }
    };

    ComponentHealth::Unhealthy { latency_ms, error }
}

// The processor task holds the only receiver, so zero means it died.
// An admin-stopped processor is still healthy; its run state is on the status API.
pub fn processor_health<T>(sender: &watch::Sender<T>) -> ComponentHealth {
    if sender.receiver_count() == 0 {
        ComponentHealth::Unhealthy {
            latency_ms: None,
            error: "Processor task is not alive".to_string(),
        }
    } else {
        ComponentHealth::Healthy { latency_ms: None }
    }
}

#[cfg(test)]
mod tests {
    use common_enums::ReconProcessorStatus;

    use super::*;

    #[test]
    fn processor_with_dropped_receiver_is_unhealthy() {
        let (sender, receiver) = watch::channel(ReconProcessorStatus::Running);
        assert!(processor_health(&sender).is_healthy());

        drop(receiver);
        assert!(!processor_health(&sender).is_healthy());
    }
}
