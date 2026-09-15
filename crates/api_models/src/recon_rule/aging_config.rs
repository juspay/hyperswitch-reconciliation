use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API request model for aging configuration
#[derive(Default, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "aging_config_type", rename_all = "snake_case")]
pub enum AgingConfigApiRequest {
    #[default]
    NoAging,
    WithThreshold {
        threshold: ThresholdApiRequest,
    },
}

/// API threshold configuration
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "threshold_type", rename_all = "snake_case")]
pub enum ThresholdApiRequest {
    WeekDays { value: u8 },
}

/// API response model for aging configuration
#[derive(Default, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "aging_config_type", rename_all = "snake_case")]
pub enum AgingConfigApiResponse {
    #[default]
    NoAging,
    WithThreshold {
        threshold: ThresholdApiResponse,
    },
}

/// API threshold configuration response
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "threshold_type", rename_all = "snake_case")]
pub enum ThresholdApiResponse {
    WeekDays { value: u8 },
}
