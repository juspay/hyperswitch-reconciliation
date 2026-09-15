use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::transformation::TransformationFailureStage;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TransformationDataV1 {
    Processed(TransformationProcessedData),
    Failed(TransformationFailureData),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TransformationProcessedData {
    pub total_count: usize,
    pub transformed_count: usize,
    pub ignored_count: usize,
    pub staging_entry_ids: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TransformationFailureData {
    pub failure_stage: TransformationFailureStage,
    pub error_message: String,
}
