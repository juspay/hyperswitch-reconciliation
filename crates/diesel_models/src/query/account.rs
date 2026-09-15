use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*, result::Error as DieselError};
use error_stack::{report, ResultExt};

use crate::{
    account::{Account, AccountNew, AccountUpdateInternal},
    errors,
    query::generics,
    schema::account::{self, dsl},
    PgPooledConn, StorageResult,
};

impl Account {
    pub async fn find_by_account_id(conn: &PgPooledConn, account_id: &str) -> StorageResult<Self> {
        generics::generic_find_by_id::<account::table, _, _>(conn, account_id.to_owned()).await
    }

    pub async fn find_by_profile_id_and_account_id(
        conn: &PgPooledConn,
        profile_id: &str,
        account_id: &str,
    ) -> StorageResult<Self> {
        let predicate = dsl::profile_id
            .eq(profile_id.to_owned())
            .and(dsl::id.eq(account_id.to_owned()));

        generics::generic_find_one::<account::table, _, _>(conn, predicate)
            .await
            .attach_printable_lazy(|| {
                format!("Error while finding account by profile_id: {profile_id} and account_id: {account_id}")
            })
    }
    pub async fn list_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::account.filter(dsl::profile_id.eq(profile_id.to_owned()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        match generics::db_metrics::track_database_call::<account::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(err) => match err {
                DieselError::NotFound => {
                    Err(report!(err)).change_context(errors::DatabaseError::NotFound)
                }
                _ => Err(report!(err)).change_context(errors::DatabaseError::Others),
            },
        }
        .attach_printable_lazy(|| {
            format!("Error while listing accounts for profile_id: {profile_id}")
        })
    }

    pub async fn find_by_profile_id_and_account_ids(
        conn: &PgPooledConn,
        profile_id: &str,
        account_ids: &[String],
    ) -> StorageResult<Vec<Self>> {
        let account_ids_owned: Vec<String> = account_ids.to_vec();
        let query = dsl::account
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::id.eq_any(account_ids_owned));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<account::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while finding accounts for profile_id: {profile_id} and account_ids: {account_ids:?}")
        })
    }

    pub async fn update_by_account_id(
        conn: &PgPooledConn,
        account_id: &str,
        account_update: AccountUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<account::table, _, _, _>(
            conn,
            dsl::id.eq(account_id.to_owned()),
            account_update,
        )
        .await
    }
}

impl AccountNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Account> {
        generics::generic_insert(conn, self).await
    }
}
