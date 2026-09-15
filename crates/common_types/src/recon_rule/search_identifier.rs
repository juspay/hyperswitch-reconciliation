use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::recon_rule::entry_field::EntryField;

pub use crate::composite_field::CompositeDelimiter;

pub mod archived;

pub use archived::SearchIdentifierV1;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "search_version", rename_all = "snake_case")]
pub enum SearchIdentifier {
    V1(SearchIdentifierV1),
    V2(SearchIdentifierV2),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "search_v2_type", rename_all = "snake_case")]
pub enum SearchIdentifierV2 {
    Single(SingleSearchIdentifier),
    Composite(CompositeSearchIdentifier),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SingleSearchIdentifier {
    pub key: SearchIdentifierKey,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CompositeSearchIdentifier {
    pub keys: Vec<SearchIdentifierKey>,
    pub delimiter: CompositeDelimiter,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct SearchIdentifierKey {
    pub source_field: EntryField,
    pub target_field: EntryField,
}

common_utils::impl_to_sql_from_sql_json!(SearchIdentifier);
