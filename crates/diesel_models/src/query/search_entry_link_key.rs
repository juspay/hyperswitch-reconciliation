use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::{EntryLinkKeyStatus, EntryLinkSide};
use common_types::field_path_key::FieldPathKey;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    errors::DatabaseError,
    query::generics,
    schema::search_entry_link_key::{self, dsl},
    search_entry_link_key::{
        SearchEntryLinkKey, SearchEntryLinkKeyNew, SearchEntryLinkKeyUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

impl SearchEntryLinkKeyNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<SearchEntryLinkKey> {
        generics::generic_insert(conn, self).await
    }
}

impl SearchEntryLinkKey {
    pub async fn bulk_update_by_ids(
        conn: &PgPooledConn,
        ids: Vec<String>,
        search_entry_link_key_update: SearchEntryLinkKeyUpdateInternal,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_update_with_results::<search_entry_link_key::table, _, _, _>(
            conn,
            dsl::id.eq_any(ids.clone()),
            search_entry_link_key_update,
        )
        .await
        .attach_printable_lazy(|| {
            format!("Error while bulk updating search entry link keys for ids: {ids:?}")
        })
    }

    pub async fn list_for_linking(
        conn: &PgPooledConn,
        link_config_id: &str,
        target_side: &EntryLinkSide,
        target_key: &FieldPathKey,
        hashed_value: &str,
        statuses: Vec<EntryLinkKeyStatus>,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::search_entry_link_key
            .filter(dsl::link_config_id.eq(link_config_id.to_owned()))
            .filter(dsl::target_side.eq(*target_side))
            .filter(dsl::target_key.eq(target_key.clone()))
            .filter(dsl::hashed_value.eq(hashed_value.to_owned()))
            .filter(dsl::status.eq_any(statuses));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_entry_link_key::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing search entry link keys for link_config_id: {link_config_id}, target_side: {target_side}, target_key: {target_key}"
            )
        })
    }
}
