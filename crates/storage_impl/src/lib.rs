pub mod account;
pub mod api_key;
pub mod business_profile;
pub mod entry;
pub mod entry_link;
pub mod ingestion_config;
pub mod ingestion_history;
pub mod link_config;
pub mod merchant_account;
pub mod metadata_schema;
pub mod recon_rule;
pub mod recon_rule_account;
pub mod search_duplicate;
pub mod search_entry_key;
pub mod search_entry_link_key;
pub mod search_expectation;
pub mod search_group;
pub mod staging_entry;
pub mod transaction;
pub mod transformation;

use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use diesel::PgConnection;
use diesel_models::{
    errors::DatabaseError, PgPooledConn as AsyncPgConn, ReadConn, StorageResult, WriteConn,
};
use error_stack::{report, ResultExt};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn<'a> =
    bb8::PooledConnection<'a, async_bb8_diesel::ConnectionManager<PgConnection>>;

/// Handed out by `DatabaseRead`; only read queries accept it.
pub struct ReadConnection<'a>(PgPooledConn<'a>);

pub struct WriteConnection<'a>(PgPooledConn<'a>);

impl ReadConn for ReadConnection<'_> {
    fn as_read_conn(&self) -> &AsyncPgConn {
        self.0.as_async_conn()
    }
}

impl ReadConn for WriteConnection<'_> {
    fn as_read_conn(&self) -> &AsyncPgConn {
        self.0.as_async_conn()
    }
}

impl WriteConn for WriteConnection<'_> {
    fn as_write_conn(&self) -> &AsyncPgConn {
        self.0.as_async_conn()
    }
}

#[async_trait]
pub trait DatabaseRead {
    async fn read_conn(&self) -> StorageResult<ReadConnection<'_>>;
}

#[async_trait]
pub trait DatabaseWrite: DatabaseRead {
    async fn write_conn(&self) -> StorageResult<WriteConnection<'_>>;
}

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

// Reads and writes both use the master pool until a read replica is added.
#[async_trait]
impl DatabaseRead for Store {
    async fn read_conn(&self) -> StorageResult<ReadConnection<'_>> {
        self.get_connection().await.map(ReadConnection)
    }
}

#[async_trait]
impl DatabaseWrite for Store {
    async fn write_conn(&self) -> StorageResult<WriteConnection<'_>> {
        self.get_connection().await.map(WriteConnection)
    }
}
