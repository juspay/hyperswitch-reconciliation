use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{composite_field::CompositeDelimiter, recon_rule::entry_field::EntryField};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "link_search_version", rename_all = "snake_case")]
pub enum LinkSearchIdentifier {
    V1(LinkSearchIdentifierV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "matching_key_type", rename_all = "snake_case")]
pub enum LinkSearchIdentifierV1 {
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

common_utils::impl_to_sql_from_sql_json!(LinkSearchIdentifier);
