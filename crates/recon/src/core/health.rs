use actix_web::http::StatusCode;
use api_models::health::DeepHealthResponse;

use crate::{
    routes::AppState,
    types::{ApplicationResponse, ReconResponse},
    utils::health::{processor_health, run_check, HealthComponent},
};

pub async fn deep_health_check(state: AppState) -> ReconResponse<DeepHealthResponse> {
    let (database, encryption, user_client) = futures::join!(
        run_check(HealthComponent::Database, state.store.health_check()),
        run_check(
            HealthComponent::Encryption,
            state.encryption_client.health_check()
        ),
        run_check(
            HealthComponent::UserClient,
            state.user_client.health_check()
        ),
    );
    let recon_processor = processor_health(&state.recon_processor_status);
    let linking_processor = processor_health(&state.linking_processor_status);

    let all_healthy = [
        &database,
        &encryption,
        &user_client,
        &recon_processor,
        &linking_processor,
    ]
    .iter()
    .all(|component| component.is_healthy());

    Ok(ApplicationResponse::Json {
        data: DeepHealthResponse {
            database,
            encryption,
            user_client,
            recon_processor,
            linking_processor,
        },
        status: if all_healthy {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
    })
}
