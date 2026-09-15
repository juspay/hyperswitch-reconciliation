use async_bb8_diesel::AsyncRunQueryDsl;
use common_types::transformation::ListTransformationConfigsFilters;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    errors,
    query::generics,
    schema::transformation_config::{self, dsl},
    transformation_config::{
        TransformationConfig, TransformationConfigNew, TransformationConfigUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

impl TransformationConfigNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<TransformationConfig> {
        generics::generic_insert(conn, self).await
    }
}

impl TransformationConfig {
    pub async fn find_by_id(conn: &PgPooledConn, transformation_id: &str) -> StorageResult<Self> {
        generics::generic_find_by_id::<transformation_config::table, _, _>(
            conn,
            transformation_id.to_owned(),
        )
        .await
    }

    pub async fn find_by_id_and_profile_id(
        conn: &PgPooledConn,
        transformation_id: &str,
        profile_id: &str,
    ) -> StorageResult<Self> {
        let predicate = dsl::profile_id
            .eq(profile_id.to_owned())
            .and(dsl::id.eq(transformation_id.to_owned()));

        generics::generic_find_one::<transformation_config::table, _, _>(conn, predicate)
            .await
            .attach_printable_lazy(|| {
                format!("Error while finding transformation config by profile_id: {profile_id} and transformation_id: {transformation_id}")
            })
    }

    pub async fn list_by_ids_and_profile_id(
        conn: &PgPooledConn,
        ids: Vec<String>,
        profile_id: &str,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::transformation_config
            .filter(dsl::id.eq_any(ids.clone()))
            .filter(dsl::profile_id.eq(profile_id.to_owned()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<transformation_config::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing transformation configs by ids: {ids:?} and profile_id: {profile_id}")
        })
    }

    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
        filters: ListTransformationConfigsFilters,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::transformation_config
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .into_boxed();

        // Apply filters
        if let Some(ingestion_id) = filters.ingestion_id {
            query = query.filter(dsl::ingestion_config_id.eq(ingestion_id));
        }

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

        query = query.order(dsl::created_at.desc());

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<transformation_config::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing transformation configs for profile_id: {profile_id}")
        })
    }

    pub async fn list_by_ingestion_id(
        conn: &PgPooledConn,
        ingestion_id: &str,
        is_active: Option<bool>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::transformation_config
            .filter(dsl::ingestion_config_id.eq(ingestion_id.to_owned()))
            .into_boxed();

        if let Some(active) = is_active {
            query = query.filter(dsl::is_active.eq(active));
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<transformation_config::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing transformation configs for ingestion_id: {ingestion_id}")
        })
    }

    pub async fn update_by_id(
        conn: &PgPooledConn,
        transformation_id: &str,
        transformation_config_update: TransformationConfigUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<transformation_config::table, _, _, _>(
            conn,
            transformation_id.to_owned(),
            transformation_config_update,
        )
        .await
        .attach_printable_lazy(|| {
            format!("Error while updating transformation config with id: {transformation_id}")
        })
    }
}
