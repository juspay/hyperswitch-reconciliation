use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::{entry_field::EntryField, trigger::TriggerOperator};

/// V1: Field-based trigger condition using type-safe EntryField enum
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TriggerV1 {
    pub field: EntryField,
    pub operator: TriggerOperator,
    pub value: String,
}
