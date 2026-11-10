use common_enums::HealthComponent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ComponentHealth {
    Healthy {
        component: HealthComponent,
    },
    Unhealthy {
        component: HealthComponent,
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeepHealthResponse {
    pub components: Vec<ComponentHealth>,
}

impl DeepHealthResponse {
    pub fn is_healthy(&self) -> bool {
        self.components
            .iter()
            .all(|health| matches!(health, ComponentHealth::Healthy { .. }))
    }
}
