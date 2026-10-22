pub mod account;
pub mod api_key;
pub mod business_profile;
pub mod entry;
pub mod generics;
pub mod ingestion_config;
pub mod ingestion_history;
pub mod merchant_account;
pub mod metadata_schema;
pub mod recon_rule;
pub mod recon_rule_account;
pub mod search_duplicate;
pub mod search_entry_key;
pub mod search_expectation;
pub mod search_group;
pub mod staging_entry;
pub mod transaction;
pub mod transformation_config;
pub mod transformation_history;

use diesel::sql_types::{Text, Timestamp};

use crate::schema::{entry as entry_schema, transaction as transaction_schema};

diesel::define_sql_function! {
    /// Postgres `date_trunc(field, source)` — truncates a timestamp to the given precision.
    fn date_trunc(field: Text, source: Timestamp) -> Timestamp;
}

diesel::allow_columns_to_appear_in_same_group_by_clause!(
    transaction_schema::rule_id,
    transaction_schema::status,
    entry_schema::account_id,
    entry_schema::currency,
);
