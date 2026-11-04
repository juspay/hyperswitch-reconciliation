use async_trait::async_trait;
use diesel_models::{
    connection::{ReadConnection, WriteConnection},
    errors::DatabaseError,
    StorageResult,
};
use error_stack::{report, ResultExt};

use super::store::{PgPool, PgPooledConn, Store};

/// A store that can hand out read connections.
#[async_trait]
pub trait DatabaseRead: Send + Sync {
    async fn read_conn(&self) -> StorageResult<ReadConn<'_>>;
}

/// A store that can hand out write connections.
#[async_trait]
pub trait DatabaseWrite: DatabaseRead {
    async fn write_conn(&self) -> StorageResult<WriteConn<'_>>;
}

/// A pooled connection for reads. Only the store hands these out.
pub struct ReadConn<'a>(PgPooledConn<'a>);

/// A pooled connection for writes. Only the store hands these out.
pub struct WriteConn<'a>(PgPooledConn<'a>);

impl ReadConnection for ReadConn<'_> {
    fn async_conn(&self) -> &diesel_models::PgPooledConn {
        &self.0
    }
}

impl ReadConnection for WriteConn<'_> {
    fn async_conn(&self) -> &diesel_models::PgPooledConn {
        &self.0
    }
}

impl WriteConnection for WriteConn<'_> {}

// Reads and writes both use the master pool until a read replica is added.
#[async_trait]
impl DatabaseRead for Store {
    async fn read_conn(&self) -> StorageResult<ReadConn<'_>> {
        pg_connection(self.get_master_pool()).await.map(ReadConn)
    }
}

#[async_trait]
impl DatabaseWrite for Store {
    async fn write_conn(&self) -> StorageResult<WriteConn<'_>> {
        pg_connection(self.get_master_pool()).await.map(WriteConn)
    }
}

async fn pg_connection(pool: &PgPool) -> StorageResult<PgPooledConn<'_>> {
    pool.get()
        .await
        .map_err(|error| report!(error))
        .change_context(DatabaseError::DatabaseConnectionError)
        .attach_printable("Failed to get database connection from pool")
}
