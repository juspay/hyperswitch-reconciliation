use async_bb8_diesel::AsyncRunQueryDsl;
use common_types::transformation::ListTransformationHistoryFilters;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::report;

use crate::{
    errors,
    query::generics,
    schema::{
        transformation_config,
        transformation_history::{self, dsl},
    },
    transformation_config::TransformationConfig,
    transformation_history::{
        TransformationConfigHistoryView, TransformationHistory, TransformationHistoryNew,
        TransformationHistoryUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

impl TransformationHistoryNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<TransformationHistory> {
        generics::generic_insert(conn, self).await
    }
}

impl TransformationHistory {
    pub async fn find_config_and_history_by_id(
        conn: &PgPooledConn,
        id: &str,
    ) -> StorageResult<TransformationConfigHistoryView> {
        let query = dsl::transformation_history
            .inner_join(
                transformation_config::table
                    .on(dsl::transformation_config_id.eq(transformation_config::id)),
            )
            .filter(dsl::id.eq(id.to_owned()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        query
            .get_result_async::<(Self, TransformationConfig)>(conn)
            .await
            .map(|(history, config)| TransformationConfigHistoryView { config, history })
            .map_err(|err| {
                report!(errors::DatabaseError::Others).attach_printable(format!(
                    "Error while fetching transformation history for id {id}: {err}"
                ))
            })
    }

    pub async fn update_by_id(
        conn: &PgPooledConn,
        transformation_history_id: &str,
        transformation_history_update: TransformationHistoryUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<transformation_history::table, _, _, _>(
            conn,
            transformation_history_id.to_owned(),
            transformation_history_update,
        )
        .await
    }

    pub async fn list_by_transformation_id(
        conn: &PgPooledConn,
        transformation_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::transformation_history
            .filter(dsl::transformation_config_id.eq(transformation_id.to_owned()))
            .order(dsl::created_at.desc())
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<transformation_history::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .map_err(|err| {
            report!(errors::DatabaseError::Others).attach_printable(format!(
                "Error while listing transformation history by transformation_id: {err}"
            ))
        })
    }

    pub async fn list_history_with_config_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
        filters: ListTransformationHistoryFilters,
    ) -> StorageResult<Vec<TransformationConfigHistoryView>> {
        let mut query = transformation_history::table
            .inner_join(
                transformation_config::table
                    .on(dsl::transformation_config_id.eq(transformation_config::dsl::id)),
            )
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .into_boxed();

        // Apply filters
        if let Some(transformation_id) = filters.transformation_id {
            query = query.filter(dsl::transformation_config_id.eq(transformation_id));
        }

        if let Some(ingestion_history_id) = filters.ingestion_history_id {
            query = query.filter(dsl::ingestion_history_id.eq(ingestion_history_id));
        }

        if let Some(account_id) = filters.account_id {
            query = query.filter(dsl::account_id.eq(account_id));
        }

        if let Some(statuses) = filters.status {
            query = query.filter(dsl::status.eq_any(statuses));
        }

        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::created_at.ge(time_range.start_time));
            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::created_at.le(end_time));
            }
        }

        query = query.order(dsl::created_at.desc());

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        query
            .get_results_async::<(Self, TransformationConfig)>(conn)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|(history, config)| TransformationConfigHistoryView { config, history })
                    .collect()
            })
            .map_err(|err| {
                report!(errors::DatabaseError::Others).attach_printable(format!(
                    "Error while listing transformation history for profile_id: {profile_id}: {err}"
                ))
            })
    }
}
