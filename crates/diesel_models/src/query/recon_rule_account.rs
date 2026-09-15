use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::ReconRuleAccountType;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    errors,
    query::generics,
    recon_rule_account::{ReconRuleAccount, ReconRuleAccountNew},
    schema::recon_rule_account::{self, dsl},
    PgPooledConn, StorageResult,
};

impl ReconRuleAccountNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ReconRuleAccount> {
        generics::generic_insert(conn, self).await
    }
}

impl ReconRuleAccount {
    pub async fn list_by_account_id_and_account_type(
        conn: &PgPooledConn,
        account_id: &str,
        account_type: ReconRuleAccountType,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::recon_rule_account
            .filter(dsl::account_id.eq(account_id.to_owned()))
            .filter(dsl::account_type.eq(account_type));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<recon_rule_account::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing recon rule accounts for account_id: {account_id} and account_type: {account_type}"
            )
        })
    }
}
