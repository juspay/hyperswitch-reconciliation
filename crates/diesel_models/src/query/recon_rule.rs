use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    errors::DatabaseError,
    query::generics,
    recon_rule::{ReconRule, ReconRuleNew, ReconRuleUpdateInternal},
    schema::recon_rule::{self, dsl},
    PgPooledConn, StorageResult,
};

impl ReconRule {
    pub async fn find_by_rule_id(conn: &PgPooledConn, rule_id: &str) -> StorageResult<Self> {
        generics::generic_find_by_id::<recon_rule::table, _, _>(conn, rule_id.to_owned()).await
    }

    pub async fn find_by_profile_id_and_rule_id(
        conn: &PgPooledConn,
        profile_id: &str,
        rule_id: &str,
    ) -> StorageResult<Self> {
        let predicate = dsl::profile_id
            .eq(profile_id.to_owned())
            .and(dsl::id.eq(rule_id.to_owned()));

        generics::generic_find_one::<recon_rule::table, _, _>(conn, predicate)
            .await
            .attach_printable_lazy(|| {
                format!(
                    "Error while finding recon rule by profile_id: {profile_id} and rule_id: {rule_id}"
                )
            })
    }

    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
        is_active: Option<bool>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::recon_rule
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .into_boxed();

        if let Some(active_filter) = is_active {
            query = query.filter(dsl::is_active.eq(active_filter));
        }

        let query = query.order(dsl::priority.asc());

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<recon_rule::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing recon rules for profile_id: {profile_id}")
        })
    }

    pub async fn list_by_rule_ids(
        conn: &PgPooledConn,
        profile_id: &str,
        rule_ids: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::recon_rule
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::id.eq_any(rule_ids.clone()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<recon_rule::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while fetching recon rules for profile_id: {profile_id} and rule_ids: {rule_ids:?}"
            )
        })
    }

    pub async fn update_by_rule_id(
        conn: &PgPooledConn,
        rule_id: &str,
        recon_rule_update: ReconRuleUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_by_id::<recon_rule::table, _, _, _>(
            conn,
            rule_id.to_owned(),
            recon_rule_update,
        )
        .await
        .attach_printable_lazy(|| {
            format!("Error while updating recon rule with rule_id: {rule_id}")
        })
    }
}

impl ReconRuleNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ReconRule> {
        generics::generic_insert(conn, self).await
    }
}
