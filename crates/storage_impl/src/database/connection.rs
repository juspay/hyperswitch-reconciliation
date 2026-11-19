use diesel_models::{
    connection::{ReadConnection, WriteConnection},
    errors::DatabaseError,
    StorageResult,
};
use error_stack::{Report, ResultExt};

use super::store::{PgPool, PgPooledConn, Store};

/// A pooled connection for reads. Only the store hands these out.
pub struct ReadConn<'a>(PgPooledConn<'a>);

/// A pooled connection for writes. Only the store hands these out.
pub struct WriteConn<'a>(PgPooledConn<'a>);

impl ReadConnection for ReadConn<'_> {
    fn inner(&self) -> &diesel_models::PgPooledConn {
        &self.0
    }
}

impl ReadConnection for WriteConn<'_> {
    fn inner(&self) -> &diesel_models::PgPooledConn {
        &self.0
    }
}

impl WriteConnection for WriteConn<'_> {}

// `Store` reads and writes on the master pool. Replica reads will go through a separate
// read-only store.
impl Store {
    /// Gets a connection for read queries.
    pub async fn read_conn(&self) -> StorageResult<ReadConn<'_>> {
        pg_connection(self.get_master_pool()).await.map(ReadConn)
    }

    /// Gets a connection for write queries. It can also read.
    pub async fn write_conn(&self) -> StorageResult<WriteConn<'_>> {
        pg_connection(self.get_master_pool()).await.map(WriteConn)
    }
}

async fn pg_connection(pool: &PgPool) -> StorageResult<PgPooledConn<'_>> {
    pool.get()
        .await
        .map_err(Report::new)
        .change_context(DatabaseError::DatabaseConnectionError)
        .attach("Failed to get database connection from pool")
}
