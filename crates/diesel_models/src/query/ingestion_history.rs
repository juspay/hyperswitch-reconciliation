use async_bb8_diesel::AsyncRunQueryDsl;
use common_types::ingestion::ListIngestionHistoryFilters;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*, result::Error as DieselError};
use error_stack::report;

use crate::{
    errors,
    ingestion_config::IngestionConfig,
    ingestion_history::{
        IngestionConfigHistoryView, IngestionHistory, IngestionHistoryNew,
        IngestionHistoryUpdateInternal,
    },
    query::generics,
    schema::{
        ingestion_config,
        ingestion_history::{self, dsl},
    },
    PgPooledConn, StorageResult,
};

impl IngestionHistory {
    pub async fn find_by_id_and_profile(
        conn: &PgPooledConn,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<ingestion_history::table, _, _>(
            conn,
            dsl::id
                .eq(id.to_owned())
                .and(dsl::profile_id.eq(profile_id.to_owned())),
        )
        .await
    }

    pub async fn find_config_and_history_by_id(
        conn: &PgPooledConn,
        id: &str,
    ) -> StorageResult<IngestionConfigHistoryView> {
        let query = dsl::ingestion_history
            .inner_join(
                ingestion_config::table.on(dsl::ingestion_config_id.eq(ingestion_config::id)),
            )
            .filter(dsl::id.eq(id.to_owned()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        query
            .get_result_async::<(Self, IngestionConfig)>(conn)
            .await
            .map(|(history, config)| IngestionConfigHistoryView { config, history })
            .map_err(|err| match err {
                DieselError::NotFound => {
                    report!(err).change_context(errors::DatabaseError::NotFound)
                }
                _ => report!(err).change_context(errors::DatabaseError::Others),
            })
    }

    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        auth_profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::ingestion_history
            .filter(dsl::profile_id.eq(auth_profile_id.to_owned()))
            .order(dsl::created_at.desc())
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<ingestion_history::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|err| {
            report!(errors::DatabaseError::Others)
                .attach_printable(format!("Error while listing ingestion history: {err}"))
        })
    }

    pub async fn list_by_ingestion_id(
        conn: &PgPooledConn,
        ingestion_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::ingestion_history
            .filter(dsl::ingestion_config_id.eq(ingestion_id.to_owned()))
            .filter(dsl::discarded_at.is_null())
            .order(dsl::created_at.desc())
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<ingestion_history::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|err| {
            report!(errors::DatabaseError::Others).attach_printable(format!(
                "Error while listing ingestion history by ingestion_id: {err}"
            ))
        })
    }

    pub async fn list_config_and_history_by_profile_id(
        conn: &PgPooledConn,
        auth_profile_id: &str,
        filters: ListIngestionHistoryFilters,
    ) -> StorageResult<Vec<IngestionConfigHistoryView>> {
        let mut query = dsl::ingestion_history
            .inner_join(
                ingestion_config::table.on(dsl::ingestion_config_id.eq(ingestion_config::id)),
            )
            .filter(dsl::profile_id.eq(auth_profile_id.to_owned()))
            .order(dsl::created_at.desc())
            .into_boxed();

        if let Some(status) = filters.status {
            query = query.filter(dsl::status.eq_any(status));
        }

        if let Some(account_id) = filters.account_id {
            query = query.filter(dsl::account_id.eq(account_id));
        }

        if let Some(ingestion_history_id) = filters.ingestion_history_id {
            query = query.filter(dsl::ingestion_history_id.eq(ingestion_history_id));
        }

        if let Some(ingestion_id) = filters.ingestion_id {
            query = query.filter(dsl::ingestion_config_id.eq(ingestion_id));
        }

        // Apply time range filter if present
        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::created_at.le(end_time));
            }
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        query
            .get_results_async::<(Self, IngestionConfig)>(conn)
            .await
            .map(|results| {
                results
                    .into_iter()
                    .map(|(history, config)| IngestionConfigHistoryView { config, history })
                    .collect()
            })
            .map_err(|err| {
                report!(errors::DatabaseError::Others)
                    .attach_printable(format!("Error while listing ingestion history: {err}"))
            })
    }

    pub async fn insert(conn: &PgPooledConn, new: IngestionHistoryNew) -> StorageResult<Self> {
        generics::generic_insert::<ingestion_history::table, _, _>(conn, new).await
    }

    pub async fn update_by_id(
        conn: &PgPooledConn,
        id: &str,
        update: IngestionHistoryUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<ingestion_history::table, _, _, Self>(
            conn,
            id.to_owned(),
            update,
        )
        .await
    }
}

impl IngestionHistoryNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<IngestionHistory> {
        generics::generic_insert(conn, self).await
    }
}
