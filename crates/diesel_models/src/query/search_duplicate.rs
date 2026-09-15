use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::SearchDuplicateStatus;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    errors,
    query::generics,
    schema::search_duplicate::{self, dsl},
    search_duplicate::{SearchDuplicate, SearchDuplicateNew, SearchDuplicateUpdateInternal},
    PgPooledConn, StorageResult,
};

impl SearchDuplicate {
    pub async fn find_by_id(
        conn: &PgPooledConn,
        search_duplicate_id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<search_duplicate::table, _, _>(conn, search_duplicate_id)
            .await
    }

    pub async fn list_by_staging_entry_id(
        conn: &PgPooledConn,
        staging_entry_id: &str,
        status_filter: Option<SearchDuplicateStatus>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::search_duplicate
            .filter(dsl::logical_staging_entry_id.eq(staging_entry_id.to_owned()))
            .into_boxed();

        if let Some(status) = status_filter {
            query = query.filter(dsl::status.eq(status));
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_duplicate::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing search duplicates for staging_entry_id: {staging_entry_id}"
            )
        })
    }

    pub async fn list_by_transformation_id_field_path_hashed_field_value(
        conn: &PgPooledConn,
        transformation_id: &str,
        field_path: &str,
        hashed_field_value: &str,
        status_filter: Option<SearchDuplicateStatus>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::search_duplicate
            .filter(dsl::transformation_config_id.eq(transformation_id.to_owned()))
            .filter(dsl::field_path.eq(field_path.to_owned()))
            .filter(dsl::hashed_field_value.eq(hashed_field_value.to_owned()))
            .into_boxed();

        if let Some(status) = status_filter {
            query = query.filter(dsl::status.eq(status));
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_duplicate::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing search duplicates for transformation_id: {transformation_id}, field_path: {field_path}, hashed_field_value: [REDACTED]"
            )
        })
    }

    pub async fn update_by_primary_staging_entry_id(
        conn: &PgPooledConn,
        primary_staging_entry_id: &str,
        search_duplicate_update: SearchDuplicateUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<search_duplicate::table, _, _, _>(
            conn,
            dsl::primary_staging_entry_id.eq(primary_staging_entry_id.to_owned()),
            search_duplicate_update,
        )
        .await
        .attach_printable_lazy(|| format!("Error while updating search duplicates for primary_staging_entry_id: {primary_staging_entry_id}"))
    }
}

impl SearchDuplicateNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<SearchDuplicate> {
        generics::generic_insert(conn, self).await
    }

    pub async fn batch_insert(
        conn: &PgPooledConn,
        search_duplicates: Vec<Self>,
    ) -> StorageResult<Vec<SearchDuplicate>> {
        let duplicate_count = search_duplicates.len();
        let query = diesel::insert_into(search_duplicate::table).values(search_duplicates);

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_duplicate::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Insert,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while batch inserting {duplicate_count} search duplicates")
        })
    }
}
