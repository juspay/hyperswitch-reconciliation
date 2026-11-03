use std::{
    future::Future,
    time::{Duration, Instant},
};

use actix_web::http::StatusCode;
use api_models::health::{ComponentHealth, HealthStatus, ProcessorHealth};
use common_utils::logging;
use tokio::sync::watch;

use crate::types::ApplicationResponse;

const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn run_check<C: error_stack::Context>(
    component: &'static str,
    check: impl Future<Output = error_stack::Result<(), C>>,
) -> ComponentHealth {
    run_optional_check(component, async { Some(check.await) }).await
}

pub async fn run_optional_check<C: error_stack::Context>(
    component: &'static str,
    check: impl Future<Output = Option<error_stack::Result<(), C>>>,
) -> ComponentHealth {
    let start = Instant::now();
    let result = tokio::time::timeout(CHECK_TIMEOUT, check).await;
    let latency_ms = Some(u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX));

    let error = match result {
        Ok(None) => return ComponentHealth::not_applicable(),
        Ok(Some(Ok(()))) => None,
        Ok(Some(Err(report))) => {
            logging::error!(component, error = ?report, "Deep health check failed");
            Some(report.current_context().to_string())
        }
        Err(_) => {
            logging::error!(component, "Deep health check timed out");
            Some(format!("Timed out after {}s", CHECK_TIMEOUT.as_secs()))
        }
    };

    ComponentHealth {
        status: if error.is_some() {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Healthy
        },
        latency_ms,
        error,
    }
}

pub trait ProcessorStatusExt<T> {
    fn processor_health(&self, running_status: T) -> ProcessorHealth;
}

impl<T: PartialEq> ProcessorStatusExt<T> for watch::Sender<T> {
    // The processor task holds the only receiver, so zero means it died
    fn processor_health(&self, running_status: T) -> ProcessorHealth {
        let running = *self.borrow() == running_status;
        let alive = self.receiver_count() > 0;

        ProcessorHealth {
            status: if alive {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            },
            running,
            error: (!alive).then(|| "Processor task is not alive".to_string()),
        }
    }
}

pub fn overall_status(statuses: &[HealthStatus]) -> HealthStatus {
    if statuses.contains(&HealthStatus::Unhealthy) {
        HealthStatus::Unhealthy
    } else {
        HealthStatus::Healthy
    }
}

pub fn health_response<T>(status: HealthStatus, data: T) -> ApplicationResponse<T> {
    ApplicationResponse::Json {
        data,
        status: match status {
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
            HealthStatus::Healthy | HealthStatus::NotApplicable => StatusCode::OK,
        },
    }
}

#[cfg(test)]
mod tests {
    use common_enums::ReconProcessorStatus;

    use super::*;

    #[test]
    fn any_unhealthy_component_makes_overall_unhealthy() {
        assert_eq!(
            overall_status(&[HealthStatus::Healthy, HealthStatus::NotApplicable]),
            HealthStatus::Healthy
        );
        assert_eq!(
            overall_status(&[HealthStatus::Healthy, HealthStatus::Unhealthy]),
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn processor_with_dropped_receiver_is_unhealthy() {
        let (sender, receiver) = watch::channel(ReconProcessorStatus::Running);
        let alive = sender.processor_health(ReconProcessorStatus::Running);
        assert_eq!(alive.status, HealthStatus::Healthy);
        assert!(alive.running);

        drop(receiver);
        let dead = sender.processor_health(ReconProcessorStatus::Running);
        assert_eq!(dead.status, HealthStatus::Unhealthy);
    }
}
