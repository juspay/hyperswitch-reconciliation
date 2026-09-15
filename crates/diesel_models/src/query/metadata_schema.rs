use async_bb8_diesel::AsyncRunQueryDsl;
use common_types::metadata_schema::ListMetadataSchemasFilters;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    errors,
    metadata_schema::{MetadataSchema, MetadataSchemaNew, MetadataSchemaUpdateInternal},
    query::generics,
    schema::metadata_schema::{self, dsl},
    PgPooledConn, StorageResult,
};

impl MetadataSchema {
    pub async fn find_by_id_and_profile_id(
        conn: &PgPooledConn,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<Self> {
        let predicate = dsl::id
            .eq(id.to_owned())
            .and(dsl::profile_id.eq(profile_id.to_owned()));

        generics::generic_find_one::<metadata_schema::table, _, _>(conn, predicate)
            .await
            .attach_printable_lazy(|| {
                format!(
                    "Error while finding metadata schema by id: {id} and profile_id: {profile_id}"
                )
            })
    }

    pub async fn list_by_ids_and_profile_id(
        conn: &PgPooledConn,
        ids: Vec<String>,
        profile_id: &str,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::metadata_schema
            .filter(dsl::id.eq_any(ids.clone()))
            .filter(dsl::profile_id.eq(profile_id.to_owned()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<metadata_schema::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing metadata schemas by ids: {ids:?} and profile_id: {profile_id}"
            )
        })
    }

    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
        filters: ListMetadataSchemasFilters,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::metadata_schema
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .into_boxed();

        // Apply schema_id filter
        if let Some(schema_id) = filters.schema_id {
            query = query.filter(dsl::schema_id.eq(schema_id));
        }

        // Apply account_id filter
        if let Some(account_id) = filters.account_id {
            query = query.filter(dsl::account_id.eq(account_id));
        }

        // Apply time range filter
        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::created_at.le(end_time));
            }
        }

        // Order by created_at descending
        query = query.order_by(dsl::created_at.desc());

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<metadata_schema::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing metadata schemas by profile_id: {profile_id} with filters")
        })
    }
}

impl MetadataSchemaNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MetadataSchema> {
        generics::generic_insert(conn, self).await
    }
}

impl MetadataSchema {
    pub async fn update_by_id(
        conn: &PgPooledConn,
        id: &str,
        update: MetadataSchemaUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<metadata_schema::table, _, _, _>(
            conn,
            id.to_owned(),
            update,
        )
        .await
        .attach_printable_lazy(|| format!("Error while updating metadata schema with id: {id}"))
    }
}
