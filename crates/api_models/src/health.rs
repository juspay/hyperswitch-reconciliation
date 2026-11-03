use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ComponentHealth {
    Healthy {
        #[serde(skip_serializing_if = "Option::is_none")]
        latency_ms: Option<u64>,
    },
    Unhealthy {
        #[serde(skip_serializing_if = "Option::is_none")]
        latency_ms: Option<u64>,
        error: String,
    },
}

impl ComponentHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeepHealthResponse {
    pub database: ComponentHealth,
    pub encryption: ComponentHealth,
    pub user_client: ComponentHealth,
    pub recon_processor: ComponentHealth,
    pub linking_processor: ComponentHealth,
}
