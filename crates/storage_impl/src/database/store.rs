use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::PgConnection;
use diesel_models::{connection::ReadConnection, errors::DatabaseError, StorageResult};
use error_stack::{Report, ResultExt};

use super::connection::DatabaseRead;
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
                .attach("Failed to create master pool")?,
        })
    }

    pub(crate) fn get_master_pool(&self) -> &PgPool {
        &self.master_pool
    }

    pub async fn health_check(&self) -> StorageResult<()> {
        let conn = self.read_conn().await?;
        diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
            .get_result_async::<i32>(conn.async_conn())
            .await
            .map_err(Report::new)
            .change_context(DatabaseError::QueryFailed)
            .attach("Database health check query failed")?;
        Ok(())
    }
}

pub(crate) async fn diesel_make_pg_pool(database: &Database) -> StorageResult<PgPool> {
    let manager =
        async_bb8_diesel::ConnectionManager::<PgConnection>::new(database.get_database_url());
    bb8::Pool::builder()
        .build(manager)
        .await
        .map_err(Report::new)
        .change_context(DatabaseError::DatabaseConnectionError)
        .attach("Failed to create database connection pool")
}
