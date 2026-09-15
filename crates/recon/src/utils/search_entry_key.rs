use api_models::search_entry_key::SearchEntryKeyResponse;
use diesel_models::search_entry_key::SearchEntryKey;

pub trait SearchEntryKeyExt {
    type ApiModel;
    fn to_api_response(self) -> Self::ApiModel;
}

impl SearchEntryKeyExt for SearchEntryKey {
    type ApiModel = SearchEntryKeyResponse;

    fn to_api_response(self) -> Self::ApiModel {
        SearchEntryKeyResponse {
            id: self.id,
            entry_id: self.entry_id,
            primary_transaction_id: self.primary_transaction_id,
            metadata_key: self.metadata_key,
            metadata_value: self.metadata_value,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }
}
