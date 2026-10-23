use common_enums::LinkingProcessorStatus;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkingTriggerResponse {
    Success,
    Failure(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LinkingProcessorStatusResponse {
    pub status: LinkingProcessorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LinkingProcessorStatusRequest {
    pub status: LinkingProcessorStatus,
}
