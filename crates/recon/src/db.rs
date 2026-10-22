pub mod account;
pub mod api_key;
pub mod business_profile;
pub mod entry;
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
pub mod transformation;

use diesel::PgConnection;
use diesel_models::{errors::DatabaseError, StorageResult};
use error_stack::{report, ResultExt};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn<'a> =
    bb8::PooledConnection<'a, async_bb8_diesel::ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct Store {
    pub master_pool: PgPool,
}

impl Store {
    /// Get a connection from the pool
    pub async fn get_connection(&self) -> StorageResult<PgPooledConn<'_>> {
        self.master_pool
            .get()
            .await
            .map_err(|error| report!(error))
            .change_context(DatabaseError::DatabaseConnectionError)
            .attach_printable("Failed to get database connection from pool")
    }
}
