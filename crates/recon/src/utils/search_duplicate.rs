use common_utils::logging;
use diesel_models::{
    entry::Entry,
    errors::DatabaseError,
    search_duplicate::{SearchDuplicate, SearchDuplicateUpdate},
    PgPooledConn,
};
use futures::future;

pub async fn update_search_duplicates_for_entries(
    conn: &PgPooledConn,
    entries: &[Entry],
    update: &SearchDuplicateUpdate,
) -> Result<(), DatabaseError> {
    future::try_join_all(entries.iter().filter_map(|entry| {
        entry
            .primary_staging_entry_id
            .as_ref()
            .map(|staging_entry_id| async move {
                SearchDuplicate::update_by_primary_staging_entry_id(
                    conn,
                    staging_entry_id,
                    update.clone().into(),
                )
                .await
                .map_err(|e| {
                    logging::error!("Failed to update search duplicate: {:?}", e);
                    *e.current_context()
                })
            })
    }))
    .await?;
    Ok(())
}
