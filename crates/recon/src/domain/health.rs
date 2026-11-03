use std::time::Duration;

use api_models::health::ComponentHealth;
use common_enums::HealthComponent;
use common_utils::logging;
use error_stack::{report, ResultExt};

use crate::{errors::health::HealthCheckError, routes::AppState};

const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn check(state: &AppState, component: HealthComponent) -> ComponentHealth {
    let health_check = async {
        match component {
            HealthComponent::Database => state
                .store
                .health_check()
                .await
                .change_context(HealthCheckError::DatabaseUnreachable),
            HealthComponent::Encryption => state
                .encryption_client
                .health_check()
                .await
                .change_context(HealthCheckError::EncryptionUnreachable),
            HealthComponent::UserClient => state
                .user_client
                .health_check()
                .await
                .change_context(HealthCheckError::UserClientUnreachable),
            // Each processor task holds the only receiver on its channel, so zero means it died.
            // An admin-stopped processor is still healthy; its run state is on the status API.
            HealthComponent::ReconProcessor => {
                if state.recon_processor_status.receiver_count() > 0 {
                    Ok(())
                } else {
                    Err(report!(HealthCheckError::ProcessorNotAlive))
                }
            }
            HealthComponent::LinkingProcessor => {
                if state.linking_processor_status.receiver_count() > 0 {
                    Ok(())
                } else {
                    Err(report!(HealthCheckError::ProcessorNotAlive))
                }
            }
        }
    };

    let result = tokio::time::timeout(CHECK_TIMEOUT, health_check)
        .await
        .unwrap_or_else(|_| Err(report!(HealthCheckError::TimedOut)));

    match result {
        Ok(()) => ComponentHealth::Healthy { component },
        Err(report) => {
            // The full report (hosts, URLs) is logged; the response carries only the message
            logging::error!(%component, error = ?report, "Deep health check failed");
            ComponentHealth::Unhealthy {
                component,
                error: report.current_context().to_string(),
            }
        }
    }
}
