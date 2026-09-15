use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Trigger data stored in DB for manual uploads
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ManualTriggerData {
    pub file_name: String,
}
