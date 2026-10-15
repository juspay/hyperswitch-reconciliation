use async_bb8_diesel::AsyncRunQueryDsl;
use common_enums::ExpectationStatus;
use common_types::field_path_key::FieldPathKey;
use common_utils::logging;
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::ResultExt;

use crate::{
    errors::DatabaseError,
    query::generics,
    schema::search_expectation::{self, dsl},
    search_expectation::{
        SearchExpectation, SearchExpectationNew, SearchExpectationUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

impl SearchExpectation {
    pub async fn find_by_id(conn: &PgPooledConn, id: String) -> StorageResult<Self> {
        generics::generic_find_by_id::<search_expectation::table, _, _>(conn, id).await
    }

    /// List expectations for matching (uses idx_expectation_match index)
    pub async fn list_by_target_account_id_rule_id_target_key_hashed_source_value(
        conn: &PgPooledConn,
        target_account_id: &str,
        rule_id: &str,
        target_key: &FieldPathKey,
        hashed_source_value: &str,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::search_expectation
            .filter(dsl::target_account_id.eq(target_account_id.to_owned()))
            .filter(dsl::rule_id.eq(rule_id.to_owned()))
            .filter(dsl::target_key.eq(target_key.clone()))
            .filter(dsl::hashed_source_value.eq(hashed_source_value.to_owned()))
            .filter(dsl::status.eq(ExpectationStatus::Expected));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_expectation::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing expectations for matching: target_account_id={target_account_id}, rule_id={rule_id}, target_key={target_key}, hashed_source_value={hashed_source_value}"
            )
        })
    }
    /// Find expectations for multiple targets in a single query
    pub async fn find_for_multiple_targets(
        conn: &PgPooledConn,
        target_account_id: &str,
        search_criteria: &[(String, FieldPathKey, String)], // (rule_id, target_key, hashed_source_value)
    ) -> StorageResult<Vec<Self>> {
        if search_criteria.is_empty() {
            return Ok(vec![]);
        }
        let mut query = search_criteria.iter().fold(
            dsl::search_expectation.into_boxed(),
            |query, (rule_id, target_key, hashed_source_value)| {
                query.or_filter(
                    dsl::rule_id
                        .eq(rule_id.to_owned())
                        .and(dsl::target_key.eq(target_key.clone()))
                        .and(dsl::hashed_source_value.eq(hashed_source_value.to_owned())),
                )
            },
        );

        query = query.filter(
            dsl::target_account_id
                .eq(target_account_id.to_owned())
                .and(dsl::status.eq(ExpectationStatus::Expected)),
        );

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let result = generics::db_metrics::track_database_call::<search_expectation::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await;

        match result {
            Ok(expectations) => Ok(expectations),
            Err(e) => Err(e)
                .change_context(DatabaseError::Others)
                .attach_printable_lazy(|| {
                    format!(
                        "Error while finding expectations for multiple targets: target_account_id={target_account_id}, search_criteria={search_criteria:?}"
                    )
                })
        }
    }

    pub async fn batch_update_status_by_primary_transaction_ids(
        conn: &PgPooledConn,
        primary_transaction_ids: Vec<String>,
        search_expectation_update: SearchExpectationUpdateInternal,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_update_with_results::<search_expectation::table, _, _, _>(
            conn,
            dsl::primary_transaction_id.eq_any(primary_transaction_ids.clone()),
            search_expectation_update,
        )
        .await
        .attach_printable_lazy(|| {
            format!("Error while batch updating search expectation status for primary_transaction_ids: {primary_transaction_ids:?}")
        })
    }

    pub async fn find_expected_expectations_by_primary_transaction_id_and_target_account_id(
        conn: &PgPooledConn,
        primary_transaction_id: &str,
        target_account_id: &str,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::search_expectation.filter(
            dsl::primary_transaction_id
                .eq(primary_transaction_id.to_owned())
                .and(dsl::target_account_id.eq(target_account_id.to_owned()))
                .and(dsl::status.eq(ExpectationStatus::Expected)),
        );

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_expectation::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while finding search expectation by primary_transaction_id: {primary_transaction_id} and target_account_id: {target_account_id}")
        })
    }

    pub async fn list_expected_by_primary_transaction_id(
        conn: &PgPooledConn,
        primary_transaction_id: &str,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::search_expectation
            .filter(dsl::primary_transaction_id.eq(primary_transaction_id.to_owned()))
            .filter(dsl::status.eq(ExpectationStatus::Expected));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<search_expectation::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing search expectations by primary_transaction_id: {primary_transaction_id}")
        })
    }
}

impl SearchExpectationNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<SearchExpectation> {
        generics::generic_insert(conn, self).await
    }

    pub async fn batch_insert(
        search_expectations: Vec<Self>,
        conn: &PgPooledConn,
    ) -> StorageResult<Vec<SearchExpectation>> {
        let expectation_count = search_expectations.len();
        generics::db_metrics::track_database_call::<search_expectation::table, _, _>(
            diesel::insert_into(search_expectation::table)
                .values(search_expectations)
                .get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Insert,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(move || {
            format!("Error while batch inserting {expectation_count} search expectations")
        })
    }
}
