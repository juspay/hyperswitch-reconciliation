use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::SearchGroupStatus;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    connection::{ReadConnection, WriteConnection},
    errors::DatabaseError,
    query::generics,
    schema::search_group::{self, dsl},
    search_group::{SearchGroup, SearchGroupNew, SearchGroupUpdateInternal},
    StorageResult,
};

impl SearchGroup {
    pub async fn find_by_id(conn: &impl ReadConnection, id: String) -> StorageResult<Self> {
        generics::generic_find_by_id::<search_group::table, _, _>(conn, id).await
    }

    pub async fn find_by_id_and_profile_id(
        conn: &impl ReadConnection,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<Self> {
        let predicate = dsl::profile_id
            .eq(profile_id.to_owned())
            .and(dsl::id.eq(id.to_owned()));

        generics::generic_find_one::<search_group::table, _, _>(conn, predicate)
            .await
            .attach_with(|| {
                format!("Error while finding search group by profile_id: {profile_id} and id: {id}")
            })
    }

    pub async fn find_by_rule_id_and_hashed_group_value(
        conn: &impl ReadConnection,
        rule_id: &str,
        hashed_grouping_value: &str,
    ) -> StorageResult<Option<Self>> {
        let query = dsl::rule_id
            .eq(rule_id.to_owned())
            .and(dsl::hashed_grouping_value.eq(hashed_grouping_value.to_owned()))
            .and(dsl::status.eq(SearchGroupStatus::Active));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_group::table, _, _>(
            generics::generic_find_one_optional::<search_group::table, _, _>(conn, query),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_with(|| {
            format!(
                "Error finding search group: rule_id={rule_id}, hashed_value={hashed_grouping_value}"
            )
        })
    }

    pub async fn find_by_primary_transaction_id_and_profile_id(
        conn: &impl ReadConnection,
        primary_transaction_id: &str,
        profile_id: &str,
    ) -> StorageResult<Option<Self>> {
        let query = dsl::primary_transaction_id
            .eq(primary_transaction_id.to_owned())
            .and(dsl::profile_id.eq(profile_id.to_owned()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_group::table, _, _>(
            generics::generic_find_one_optional::<search_group::table, _, _>(conn, query),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_with(|| {
            format!(
                "Error finding search group by transaction and profile: primary_transaction_id={primary_transaction_id}, profile_id={profile_id}"
            )
        })
    }

    pub async fn discard_by_id(
        conn: &impl WriteConnection,
        id: &str,
        update: SearchGroupUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<search_group::table, _, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
            update,
        )
        .await
        .attach_with(|| format!("Error updating search group: id={id}"))
    }
}

impl SearchGroupNew {
    pub async fn insert(self, conn: &impl WriteConnection) -> StorageResult<SearchGroup> {
        generics::generic_insert(conn, self).await
    }

    pub async fn batch_insert(
        search_groups: Vec<Self>,
        conn: &impl WriteConnection,
    ) -> StorageResult<Vec<SearchGroup>> {
        if search_groups.is_empty() {
            return Ok(Vec::new());
        }

        let group_count = search_groups.len();
        generics::db_metrics::track_database_call::<search_group::table, _, _>(
            diesel::insert_into(search_group::table)
                .values(search_groups)
                .get_results_async(conn.async_conn()),
            generics::db_metrics::DatabaseOperation::Insert,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_with(move || format!("Error batch inserting {group_count} search groups"))
    }
}
