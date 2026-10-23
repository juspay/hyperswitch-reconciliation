use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::EntryLinkSide;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    errors::DatabaseError,
    link_config::{LinkConfig, LinkConfigNew, LinkConfigUpdateInternal},
    query::generics,
    schema::link_config::{self, dsl},
    PgPooledConn, StorageResult,
};

impl LinkConfigNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<LinkConfig> {
        generics::generic_insert(conn, self).await
    }
}

impl LinkConfig {
    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::link_config
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .order((dsl::priority.asc(), dsl::id.asc()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<link_config::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing link configs for profile_id: {profile_id}")
        })
    }

    pub async fn find_by_profile_id_and_link_config_id(
        conn: &PgPooledConn,
        profile_id: &str,
        link_config_id: &str,
    ) -> StorageResult<Self> {
        let predicate = dsl::profile_id
            .eq(profile_id.to_owned())
            .and(dsl::id.eq(link_config_id.to_owned()));

        generics::generic_find_one::<link_config::table, _, _>(conn, predicate).await
    }

    pub async fn update_by_profile_id_and_link_config_id(
        conn: &PgPooledConn,
        profile_id: &str,
        link_config_id: &str,
        update: LinkConfigUpdateInternal,
    ) -> StorageResult<Self> {
        let predicate = dsl::profile_id
            .eq(profile_id.to_owned())
            .and(dsl::id.eq(link_config_id.to_owned()));

        generics::generic_update_with_unique_predicate_get_result::<link_config::table, _, _, _>(
            conn, predicate, update,
        )
        .await
    }

    pub async fn list_active_by_acc_rule_profile(
        conn: &PgPooledConn,
        account_id: &str,
        rule_id: &str,
        profile_id: &str,
        link_side: &EntryLinkSide,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::link_config
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::account_id.eq(account_id.to_owned()))
            .filter(dsl::is_active.eq(true))
            .into_boxed();

        let query = match link_side {
            EntryLinkSide::Transaction => {
                query.filter(dsl::transaction_rule_id.eq(rule_id.to_owned()))
            }
            EntryLinkSide::Confirmation => {
                query.filter(dsl::confirmation_rule_id.eq(rule_id.to_owned()))
            }
        };

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<link_config::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing active link configs for account_id: {account_id}, rule_id: {rule_id} and profile_id: {profile_id}"
            )
        })
    }
}
