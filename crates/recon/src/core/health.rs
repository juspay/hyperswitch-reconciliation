use actix_web::http::StatusCode;
use api_models::health::DeepHealthResponse;
use common_enums::HealthComponent;
use strum::IntoEnumIterator;

use crate::{
    domain::health,
    routes::AppState,
    types::{ApplicationResponse, ReconResponse},
};

pub async fn health_check() -> ReconResponse<()> {
    Ok(ApplicationResponse::TextPlain("OK".to_string()))
}

pub async fn deep_health_check(state: AppState) -> ReconResponse<DeepHealthResponse> {
    let components = futures::future::join_all(
        HealthComponent::iter().map(|component| health::check(&state, component)),
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
