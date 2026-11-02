use async_trait::async_trait;
use diesel_models::{errors::DatabaseError, StorageResult};
use error_stack::{report, ResultExt};

use super::connection::{PgPool, PgPooledConn, ReadConnection, WriteConnection};

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
