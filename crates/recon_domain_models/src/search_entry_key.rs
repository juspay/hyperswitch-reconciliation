use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel_models::search_entry_key::SearchEntryKeyNew;

#[derive(Clone, Debug)]
pub struct SearchEntryKeyCreation {
    pub metadata_key: String,
    pub metadata_value: Option<String>,
}

impl SearchEntryKeyCreation {
    pub fn rows_from_exposed_metadata(
        exposed_metadata: &serde_json::Value,
        searchable_keys: &[String],
    ) -> Vec<Self> {
        searchable_keys
            .iter()
            .filter_map(|key| {
                exposed_metadata.get(key).map(|value| Self {
                    metadata_key: key.clone(),
                    metadata_value: searchable_value_to_string(value),
                })
            })
            .collect()
    }

    pub fn get_search_entry_key(
        self,
        profile_id: String,
        entry_id: String,
        primary_transaction_id: String,
    ) -> DomainSearchEntryKey {
        DomainSearchEntryKey {
            profile_id,
            entry_id,
            primary_transaction_id,
            metadata_key: self.metadata_key,
            metadata_value: self.metadata_value,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainSearchEntryKey {
    pub profile_id: String,
    pub entry_id: String,
    pub primary_transaction_id: String,
    pub metadata_key: String,
    pub metadata_value: Option<String>,
}

impl DomainSearchEntryKey {
    pub fn construct_new(self) -> SearchEntryKeyNew {
        let now = date_time::now();
        SearchEntryKeyNew {
            id: generate_id_with_default_len("sek"),
            profile_id: self.profile_id,
            entry_id: self.entry_id,
            primary_transaction_id: self.primary_transaction_id,
            metadata_key: self.metadata_key,
            metadata_value: self.metadata_value,
            created_at: now,
            last_modified_at: now,
        }
    }
}

/// Flattens a scalar JSON value to its text form for the searchable column.
/// Non-scalar values (arrays/objects) and null become `None`.
fn searchable_value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            None
        }
    }
}
