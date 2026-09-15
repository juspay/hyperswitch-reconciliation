use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{composite_field::CompositeDelimiter, recon_rule::entry_field::EntryField};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "grouping_field_version", rename_all = "snake_case")]
pub enum GroupingField {
    V1(GroupingFieldV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "grouping_field_v1_type", rename_all = "snake_case")]
pub enum GroupingFieldV1 {
    Single(SingleGroupingField),
    Composite(CompositeGroupingField),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SingleGroupingField {
    pub field: EntryField,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CompositeGroupingField {
    pub fields: Vec<EntryField>,
    pub delimiter: CompositeDelimiter,
}
