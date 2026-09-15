use async_bb8_diesel::AsyncRunQueryDsl;
use common_types::ingestion::ListIngestionConfigsFilters;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::{report, ResultExt};

use crate::{
    errors,
    ingestion_config::{IngestionConfig, IngestionConfigNew, IngestionConfigUpdateInternal},
    query::generics,
    schema::ingestion_config::{self, dsl},
    PgPooledConn, StorageResult,
};

impl IngestionConfig {
    pub async fn find_by_ingestion_id(
        conn: &PgPooledConn,
        ingestion_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<ingestion_config::table, _, _>(conn, ingestion_id.to_owned())
            .await
    }

    pub async fn find_by_profile_id_and_ingestion_id(
        conn: &PgPooledConn,
        profile_id: &str,
        ingestion_id: &str,
    ) -> StorageResult<Self> {
        let predicate = dsl::profile_id
            .eq(profile_id.to_owned())
            .and(dsl::id.eq(ingestion_id.to_owned()));

        generics::generic_find_one::<ingestion_config::table, _, _>(conn, predicate)
            .await
            .attach_printable_lazy(|| {
                format!("Error while finding ingestion config by profile_id: {profile_id} and ingestion_id: {ingestion_id}")
            })
    }

    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
        filters: ListIngestionConfigsFilters,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::ingestion_config
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .into_boxed();

        if let Some(account_id) = filters.account_id {
            query = query.filter(dsl::account_id.eq(account_id));
        }

        if let Some(is_active) = filters.is_active {
            query = query.filter(dsl::is_active.eq(is_active));
        }

        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::created_at.le(end_time));
            }
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<ingestion_config::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|err| {
            report!(errors::DatabaseError::Others)
                .attach_printable(format!("Error while listing ingestion configs: {err}"))
        })
    }

    pub async fn insert(conn: &PgPooledConn, new: IngestionConfigNew) -> StorageResult<Self> {
        generics::generic_insert::<ingestion_config::table, _, _>(conn, new).await
    }

    pub async fn update_by_ingestion_id(
        conn: &PgPooledConn,
        ingestion_id: &str,
        update: IngestionConfigUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<ingestion_config::table, _, _, _>(
            conn,
            ingestion_id.to_owned(),
            update,
        )
        .await
        .attach_printable_lazy(|| {
            format!("Error while updating ingestion config with ingestion_id: {ingestion_id}")
        })
    }
}
