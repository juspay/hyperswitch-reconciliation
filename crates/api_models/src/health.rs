use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ComponentHealth {
    pub fn not_applicable() -> Self {
        Self {
            status: HealthStatus::NotApplicable,
            latency_ms: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeepHealthResponse {
    pub status: HealthStatus,
    pub database: ComponentHealth,
    pub encryption: ComponentHealth,
    pub user_client: ComponentHealth,
    pub recon_processor: ProcessorHealth,
    pub linking_processor: ProcessorHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessorHealth {
    pub status: HealthStatus,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
