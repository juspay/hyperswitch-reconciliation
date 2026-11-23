use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Dependencies that `/health/deep` checks. Separate from recon's list: each binary checks
/// only what it uses.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    ToSchema,
    strum::Display,
    strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum HealthComponent {
    Database,
}

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
