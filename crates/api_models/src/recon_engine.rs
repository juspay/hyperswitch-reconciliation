use common_enums::ReconProcessorStatus;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::transaction::GetTransactionResponse;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconTriggerResponse {
    Success(Box<GetTransactionResponse>),
    Failure(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessorStatusResponse {
    pub status: ReconProcessorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessorStatusRequest {
    pub status: ReconProcessorStatus,
}
