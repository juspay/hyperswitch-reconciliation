use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use common_utils::{chunking::chunked, logging};
use diesel::{debug_query, pg::Pg, ExpressionMethods, QueryDsl};
use error_stack::ResultExt;

use crate::{
    errors::DatabaseError,
    query::generics,
    schema::search_entry_key,
    search_entry_key::{SearchEntryKey, SearchEntryKeyNew},
    PgPooledConn, StorageResult,
};

const SEARCH_ENTRY_KEY_BATCH_CHUNK_SIZE: usize = 5000;

impl SearchEntryKeyNew {
    pub async fn batch_insert(
        rows: Vec<Self>,
        conn: &PgPooledConn,
    ) -> StorageResult<Vec<SearchEntryKey>> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let row_count = rows.len();

        conn.transaction_async(|conn| async move {
            let mut inserted: Vec<SearchEntryKey> = Vec::with_capacity(row_count);

            for (chunk_idx, chunk) in chunked(rows, SEARCH_ENTRY_KEY_BATCH_CHUNK_SIZE).enumerate() {
                let chunk_len = chunk.len();
                let query = diesel::insert_into(search_entry_key::table).values(chunk);
                logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

                let chunk_rows: Vec<SearchEntryKey> =
                    generics::db_metrics::track_database_call::<search_entry_key::table, _, _>(
                        query.get_results_async(&conn),
                        generics::db_metrics::DatabaseOperation::Insert,
                    )
                    .await
                    .map_err(|e| {
                        logging::error!(
                            "Error while inserting search-entry-key chunk \
                             (chunk_idx={chunk_idx}, chunk_len={chunk_len}, total={row_count}): {e:?}",
                        );
                        DatabaseError::Others
                    })?;

                inserted.extend(chunk_rows);
            }

            Result::<_, DatabaseError>::Ok(inserted)
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Transaction failed while batch inserting {row_count} search entry key rows")
        })
    }
}

impl SearchEntryKey {
    pub async fn list_by_entry_ids_and_profile_id(
        conn: &PgPooledConn,
        entry_ids: &[String],
        profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        use search_entry_key::dsl;

        let entry_ids_owned = entry_ids.to_vec();
        let mut query = dsl::search_entry_key
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::entry_id.eq_any(entry_ids_owned))
            .into_boxed();

        if let Some(limit_value) = limit {
            query = query.limit(limit_value);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_entry_key::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing search entry keys for profile_id: {profile_id} and entry_ids: {entry_ids:?}"
            )
        })
    }
}
