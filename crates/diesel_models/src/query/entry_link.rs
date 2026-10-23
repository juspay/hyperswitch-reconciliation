use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use common_utils::{chunking::chunked, logging};
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    entry::Entry,
    entry_link::{EntryLink, EntryLinkNew},
    errors::DatabaseError,
    query::generics,
    schema::{entry, entry_link},
    PgPooledConn, StorageResult,
};

const ENTRY_LINK_BATCH_CHUNK_SIZE: usize = 1000;

impl EntryLink {
    pub async fn list_linked_entries_by_entry_id_and_profile_id(
        conn: &PgPooledConn,
        entry_id: &str,
        profile_id: &str,
    ) -> StorageResult<Vec<(Self, Entry)>> {
        let query = entry_link::table
            .inner_join(
                entry::table.on(entry_link::transaction_entry_id
                    .eq(entry_id.to_owned())
                    .and(entry_link::confirmation_entry_id.eq(entry::id))
                    .or(entry_link::confirmation_entry_id
                        .eq(entry_id.to_owned())
                        .and(entry_link::transaction_entry_id.eq(entry::id)))),
            )
            .filter(entry_link::profile_id.eq(profile_id.to_owned()))
            .order((entry_link::created_at.asc(), entry_link::id.asc()))
            .select((entry_link::all_columns, entry::all_columns));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<entry_link::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing linked entries for entry_id: {entry_id}, profile_id: {profile_id}")
        })
    }
}

impl EntryLinkNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<EntryLink> {
        generics::generic_insert(conn, self).await
    }

    pub async fn bulk_insert_chunked(
        conn: &PgPooledConn,
        entry_links: Vec<Self>,
    ) -> StorageResult<Vec<EntryLink>> {
        if entry_links.is_empty() {
            return Ok(Vec::new());
        }

        let entry_link_count = entry_links.len();

        conn.transaction_async(|conn| async move {
            let mut inserted_entry_links: Vec<EntryLink> = Vec::with_capacity(entry_link_count);

            for (chunk_idx, chunk) in
                chunked(entry_links, ENTRY_LINK_BATCH_CHUNK_SIZE).enumerate()
            {
                let chunk_len = chunk.len();
                let query = diesel::insert_into(entry_link::table).values(chunk);
                logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

                let rows: Vec<EntryLink> =
                    generics::db_metrics::track_database_call::<entry_link::table, _, _>(
                        query.get_results_async(&conn),
                        generics::db_metrics::DatabaseOperation::Insert,
                    )
                    .await
                    .map_err(|e| {
                        logging::error!(
                            "Error while inserting entry link chunk (chunk_idx={chunk_idx}, chunk_len={chunk_len}, total={entry_link_count}): {e:?}",
                        );
                        DatabaseError::Others
                    })?;

                inserted_entry_links.extend(rows);
            }

            Result::<_, DatabaseError>::Ok(inserted_entry_links)
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Transaction failed while bulk inserting {entry_link_count} entry links")
        })
    }
}
