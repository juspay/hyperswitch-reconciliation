use diesel::PgConnection;
use diesel_models::{errors::DatabaseError, StorageResult};
use error_stack::{report, ResultExt};

use super::connection::{DatabaseWrite, WriteConn};
use crate::config::Database;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn<'a> =
    bb8::PooledConnection<'a, async_bb8_diesel::ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct Store {
    master_pool: PgPool,
}

impl Store {
    pub async fn new(master: &Database) -> StorageResult<Self> {
        Ok(Self {
            master_pool: diesel_make_pg_pool(master)
                .await
                .attach_printable("Failed to create master pool")?,
        })
    }

    pub(crate) fn get_master_pool(&self) -> &PgPool {
        &self.master_pool
    }

    /// Get a write connection from the pool. Prefer `read_conn()` / `write_conn()` in new code.
    pub async fn get_connection(&self) -> StorageResult<WriteConn<'_>> {
        self.write_conn().await
    }
}

pub(crate) async fn diesel_make_pg_pool(database: &Database) -> StorageResult<PgPool> {
    let manager =
        async_bb8_diesel::ConnectionManager::<PgConnection>::new(database.get_database_url());
    bb8::Pool::builder()
        .build(manager)
        .await
        .map_err(|error| report!(error))
        .change_context(DatabaseError::DatabaseConnectionError)
        .attach_printable("Failed to create database connection pool")
}
