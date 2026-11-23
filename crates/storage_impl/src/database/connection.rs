pub use diesel_models::connection::{ReadConn, WriteConn};
use diesel_models::{connection::PooledConn, errors::DatabaseError, StorageResult};
use error_stack::{Report, ResultExt};

use super::store::{PgPool, Store};

// `Store` reads and writes on the master pool. Replica reads will go through a separate
// read-only store.
impl Store {
    /// Gets a connection for read queries.
    pub async fn read_conn(&self) -> StorageResult<ReadConn<'_>> {
        pg_connection(self.get_master_pool())
            .await
            .map(ReadConn::new)
    }

    /// Gets a connection for write queries. It can also read.
    pub async fn write_conn(&self) -> StorageResult<WriteConn<'_>> {
        pg_connection(self.get_master_pool())
            .await
            .map(WriteConn::new)
    }
}

async fn pg_connection(pool: &PgPool) -> StorageResult<PooledConn<'_>> {
    pool.get()
        .await
        .map_err(Report::new)
        .change_context(DatabaseError::DatabaseConnectionError)
        .attach("Failed to get database connection from pool")
}
