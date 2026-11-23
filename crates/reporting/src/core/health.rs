use std::time::Duration;

use actix_web::http::StatusCode;
use error_stack::{Report, ResultExt};
use recon_env::logging;
use strum::IntoEnumIterator;

use crate::{
    errors::health::HealthCheckError,
    routes::AppState,
    types::{
        health::{ComponentHealth, DeepHealthResponse, HealthComponent},
        ApplicationResponse, ReportingResponse,
    },
};

const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn health_check() -> ReportingResponse<()> {
    Ok(ApplicationResponse::TextPlain("OK".to_string()))
}

pub async fn deep_health_check(state: AppState) -> ReportingResponse<DeepHealthResponse> {
    let components = futures::future::join_all(
        HealthComponent::iter().map(|component| check_component(&state, component)),
    )
    .await;
    let response = DeepHealthResponse { components };

    let status = if response.is_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok(ApplicationResponse::Json {
        data: response,
        status,
    })
}

async fn check_component(state: &AppState, component: HealthComponent) -> ComponentHealth {
    let health_check = async {
        match component {
            HealthComponent::Database => state
                .store
                .health_check()
                .await
                .change_context(HealthCheckError::DatabaseUnreachable),
        }
    };

    let result = tokio::time::timeout(CHECK_TIMEOUT, health_check)
        .await
        .unwrap_or_else(|_| Err(Report::new(HealthCheckError::TimedOut)));

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
