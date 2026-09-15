use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SearchEntryKeyListRequest {
    pub entry_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SearchEntryKeyResponse {
    #[schema(example = "sek_1234567890")]
    pub id: String,
    #[schema(example = "entry_1234567890")]
    pub entry_id: String,
    #[schema(example = "txn_1234567890")]
    pub primary_transaction_id: String,
    #[schema(example = "transaction_id")]
    pub metadata_key: String,
    #[schema(example = "psp_ref_123")]
    pub metadata_value: Option<String>,
    #[schema(example = "2022-01-01T00:00:00")]
    pub created_at: PrimitiveDateTime,
    #[schema(example = "2022-01-01T00:00:00")]
    pub last_modified_at: PrimitiveDateTime,
}

pub type SearchEntryKeyListResponse = Vec<SearchEntryKeyResponse>;
