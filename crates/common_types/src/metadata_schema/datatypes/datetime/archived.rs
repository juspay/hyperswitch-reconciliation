use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// DateTime configuration V1
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DateTimeConfigV1 {}
