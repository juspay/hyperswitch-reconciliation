use diesel::PgConnection;
use diesel_models::{errors::DatabaseError, StorageResult};
use error_stack::{report, ResultExt};

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
