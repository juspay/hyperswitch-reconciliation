use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::EntryField;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SearchIdentifierV1 {
    pub source_field: EntryField,
    pub target_field: EntryField,
}
