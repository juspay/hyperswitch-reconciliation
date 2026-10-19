use common_types::{composite_field::CompositeDelimiter, recon_rule::entry_field::EntryField};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "matching_key_type", rename_all = "snake_case")]
pub enum LinkSearchIdentifier {
    Single(SingleLinkIdentifier),
    Composite(CompositeLinkIdentifier),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SingleLinkIdentifier {
    pub key: LinkIdentifierKey,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CompositeLinkIdentifier {
    pub keys: Vec<LinkIdentifierKey>,
    pub delimiter: CompositeDelimiter,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LinkIdentifierKey {
    pub transaction_field: EntryField,
    pub confirmation_field: EntryField,
}
