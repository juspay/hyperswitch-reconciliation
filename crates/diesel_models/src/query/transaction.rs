use std::collections::HashMap;

use crate::schema::search_entry_key;
use crate::transaction::{
    RuleAccountTransactionOverviewRow, TransactionAccountView, TransactionOverviewRow,
    TransactionRuleView, TransactionRuleViewPage, TransactionTimeBucketRow,
};
use crate::{
    errors,
    query::{date_trunc, generics},
    recon_rule::ReconRule,
    schema::{
        account, entry, recon_rule,
        transaction::{self, dsl},
    },
    transaction::{
        ExpectedSourceAppendUpdateInternal, Transaction, TransactionNew, TransactionUpdateInternal,
    },
    PgPooledConn, StorageResult,
};
use async_bb8_diesel::AsyncRunQueryDsl;
use bigdecimal::BigDecimal;
use common_enums::{BalanceDirection, Currency, DieselTransactionStatus, Granularity};
use common_types::{
    date_time::{DefinedTimeRange, TimeRange},
    pagination::{CursorDirection, PaginationQuery},
    transaction::TransactionListStatusFilter,
    transaction::{TransactionAccountFilters, TransactionListFiltersV2, TransactionListSortField},
    units::MinorUnit,
};
use common_utils::logging;
use diesel::dsl::{case_when, count, count_star, exists};
use diesel::sql_types::{Bool, Timestamp, Varchar};
use diesel::WindowExpressionMethods;
use diesel::{
    debug_query, pg::Pg, prelude::*, result::Error as DieselError, BoxableExpression, IntoSql,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

type TransactionOverviewRowTuple = (
    String,
    PrimitiveDateTime,
    DieselTransactionStatus,
    Currency,
    i64,
    Option<BigDecimal>,
    Option<BigDecimal>,
    i64,
    Option<BigDecimal>,
    Option<BigDecimal>,
);

type RuleAccountTransactionOverviewSummaryRowTuple = (
    String,
    String,
    DieselTransactionStatus,
    Currency,
    i64,
    i64,
    Option<BigDecimal>,
    Option<BigDecimal>,
    i64,
    i64,
    Option<BigDecimal>,
    Option<BigDecimal>,
);

type TransactionOverviewSummaryRowTuple = (
    String,
    DieselTransactionStatus,
    Currency,
    i64,
    Option<BigDecimal>,
    Option<BigDecimal>,
    i64,
    Option<BigDecimal>,
    Option<BigDecimal>,
);

fn transaction_is_missing_expression<QS: 'static>(
    rule_missing_threshold_map: &HashMap<String, Option<PrimitiveDateTime>>,
) -> Box<dyn BoxableExpression<QS, Pg, SqlType = Bool>>
where
    transaction::rule_id: BoxableExpression<QS, Pg, SqlType = Varchar>,
    transaction::effective_at: BoxableExpression<QS, Pg, SqlType = Timestamp>,
    transaction::status: BoxableExpression<QS, Pg, SqlType = Varchar>,
{
    let mut older_than_cutoff: Box<dyn BoxableExpression<QS, Pg, SqlType = Bool>> =
        Box::new(false.into_sql::<Bool>());

    for (rule_id, missing_before) in rule_missing_threshold_map {
        if let Some(missing_before) = missing_before {
            older_than_cutoff = Box::new(
                older_than_cutoff.or(dsl::rule_id
                    .eq(rule_id.clone())
                    .and(dsl::effective_at.lt(*missing_before))),
            );
        }
    }

    Box::new(
        dsl::status
            .eq(DieselTransactionStatus::Expected)
            .and(older_than_cutoff),
    )
}

impl Transaction {
    pub async fn find_by_id(conn: &PgPooledConn, id: String) -> StorageResult<Self> {
        generics::generic_find_one::<transaction::table, _, _>(conn, dsl::id.eq(id)).await
    }

    pub async fn find_by_id_and_profile(
        conn: &PgPooledConn,
        id: String,
        profile_id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<transaction::table, _, _>(
            conn,
            dsl::id.eq(id).and(dsl::profile_id.eq(profile_id)),
        )
        .await
    }

    pub async fn find_latest_version_by_logical_id_and_profile_id(
        conn: &PgPooledConn,
        transaction_id: &str,
        profile_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<transaction::table, _, _>(
            conn,
            dsl::transaction_id
                .eq(transaction_id.to_owned())
                .and(dsl::profile_id.eq(profile_id.to_owned()))
                .and(dsl::discarded_at.is_null()),
        )
        .await
        .attach_printable_lazy(|| {
            format!(
                "Error while finding latest transaction for transaction_id: {transaction_id}, profile_id: {profile_id}"
            )
        })
    }

    pub async fn find_by_transaction_id_and_version(
        conn: &PgPooledConn,
        transaction_id: &str,
        version: i32,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<transaction::table, _, _>(
            conn,
            dsl::transaction_id
                .eq(transaction_id.to_owned())
                .and(dsl::version.eq(version)),
        )
        .await
    }

    pub async fn list_by_transaction_id(
        conn: &PgPooledConn,
        transaction_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::transaction
            .filter(dsl::transaction_id.eq(transaction_id.to_owned()))
            .order(dsl::version.desc())
            .into_boxed();

        if let Some(limit_value) = limit {
            query = query.limit(limit_value);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        match generics::db_metrics::track_database_call::<transaction::table, _, _>(
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
            format!("Error while finding transactions for transaction_id: {transaction_id}")
        })
    }

    pub async fn list_transactions_by_primary_ids_and_profile(
        conn: &PgPooledConn,
        primary_transaction_ids: &[String],
        profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        let transaction_ids_owned = primary_transaction_ids.to_vec();
        let mut query = dsl::transaction
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::id.eq_any(transaction_ids_owned))
            .into_boxed();

        if let Some(limit_value) = limit {
            query = query.limit(limit_value);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<transaction::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while finding transactions for profile_id: {profile_id} and transaction_ids: {primary_transaction_ids:?}"
            )
        })
    }

    pub async fn update_by_id(
        conn: &PgPooledConn,
        id: &str,
        transaction_update: TransactionUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<transaction::table, _, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
            transaction_update,
        )
        .await
        .attach_printable_lazy(|| format!("Error while updating transaction for id: {id}"))
    }

    pub async fn append_expected_source_entry(
        conn: &PgPooledConn,
        id: &str,
        update: ExpectedSourceAppendUpdateInternal,
    ) -> StorageResult<Self> {
        let query = diesel::update(
            dsl::transaction.filter(
                dsl::id
                    .eq(id.to_owned())
                    .and(dsl::status.eq(DieselTransactionStatus::Expected)),
            ),
        )
        .set((
            dsl::entry_ids.eq(update.entry_ids),
            dsl::credit_amount.eq(update.credit_amount),
            dsl::debit_amount.eq(update.debit_amount),
            dsl::effective_at.eq(update.effective_at),
            dsl::last_modified_at.eq(update.last_modified_at),
        ));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<transaction::table, _, _>(
            query.get_result_async(conn),
            generics::db_metrics::DatabaseOperation::Update,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while appending source entry to expected transaction: {id}")
        })
    }

    pub async fn list_with_accounts_and_entries(
        conn: &PgPooledConn,
        profile_id: &str,
        filters: TransactionAccountFilters,
        limit: Option<i64>,
    ) -> StorageResult<Vec<TransactionAccountView>> {
        let mut query = transaction::table
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .into_boxed();

        // Apply standard transaction-level filters
        if let Some(transaction_id) = filters.transaction_id {
            query = query.filter(dsl::transaction_id.eq(transaction_id));
        }

        if let Some(statuses) = filters.status {
            query = query.filter(dsl::status.eq_any(statuses));
        }

        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::effective_at.ge(time_range.start_time));
            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::effective_at.le(end_time));
            }
        }

        if let Some(rule_id) = filters.rule_id {
            query = query.filter(dsl::rule_id.eq(rule_id));
        }

        // Credit account filter: "Show transactions that have at least one credit entry to these accounts"
        if let Some(credit_accounts) = filters.credit_account {
            let credit_exists = exists(
                entry::table
                    .filter(entry::logical_transaction_id.eq(dsl::transaction_id))
                    .filter(entry::transaction_version.eq(dsl::version))
                    .filter(entry::balance_direction.eq(BalanceDirection::Credit))
                    .filter(entry::account_id.eq_any(credit_accounts))
                    .select(entry::id),
            );
            query = query.filter(credit_exists);
        }

        // Debit account filter: "Show transactions that have at least one debit entry from these accounts"
        if let Some(debit_accounts) = filters.debit_account {
            let debit_exists = exists(
                entry::table
                    .filter(entry::logical_transaction_id.eq(dsl::transaction_id))
                    .filter(entry::transaction_version.eq(dsl::version))
                    .filter(entry::balance_direction.eq(BalanceDirection::Debit))
                    .filter(entry::account_id.eq_any(debit_accounts))
                    .select(entry::id),
            );
            query = query.filter(debit_exists);
        }

        query = query.order(dsl::effective_at.desc());

        if let Some(limit_value) = limit {
            query = query.limit(limit_value);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let results = generics::db_metrics::track_database_call::<transaction::table, _, _>(
            query
                .inner_join(
                    entry::table.on(entry::logical_transaction_id
                        .eq(dsl::transaction_id)
                        .and(entry::transaction_version.eq(dsl::version))),
                )
                .inner_join(account::table.on(account::id.eq(entry::account_id)))
                .inner_join(recon_rule::table.on(recon_rule::id.eq(dsl::rule_id)))
                .select((
                    transaction::all_columns,
                    entry::all_columns,
                    account::all_columns,
                    recon_rule::all_columns,
                ))
                .get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while listing transactions with accounts for profile_id: {profile_id}")
        })?;

        Ok(results
            .into_iter()
            .map(
                |(transaction, entry, account, recon_rule)| TransactionAccountView {
                    transaction,
                    entry,
                    account,
                    recon_rule,
                },
            )
            .collect())
    }

    pub async fn aggregate_status_counts_by_rule(
        conn: &PgPooledConn,
        profile_id: &str,
        time_range: TimeRange,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<TransactionOverviewRow>> {
        let mut query = transaction::table
            .group_by((dsl::rule_id, dsl::status, dsl::currency))
            .select((
                dsl::rule_id,
                dsl::status,
                dsl::currency,
                count_star(),
                diesel::dsl::sum(dsl::credit_amount),
                diesel::dsl::sum(dsl::debit_amount),
                count(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map),
                    dsl::id,
                )),
                diesel::dsl::sum(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map),
                    dsl::credit_amount,
                )),
                diesel::dsl::sum(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map),
                    dsl::debit_amount,
                )),
            ))
            .into_boxed()
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::status.ne(DieselTransactionStatus::Archived));

        query = query.filter(dsl::effective_at.ge(time_range.start_time));
        if let Some(end_time) = time_range.end_time {
            query = query.filter(dsl::effective_at.le(end_time));
        }

        if let Some(rule_ids) = rule_ids {
            query = query.filter(dsl::rule_id.eq_any(rule_ids));
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let rows: Vec<TransactionOverviewSummaryRowTuple> =
            generics::db_metrics::track_database_call::<transaction::table, _, _>(
                query.get_results_async(conn),
                generics::db_metrics::DatabaseOperation::Filter,
            )
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| {
                format!(
                    "Error while aggregating transaction status counts for profile_id: {profile_id}"
                )
            })?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    rule_id,
                    status,
                    currency,
                    total_count,
                    total_credit,
                    total_debit,
                    missing_count,
                    missing_credit,
                    missing_debit,
                )| TransactionOverviewRow {
                    rule_id,
                    status,
                    currency,
                    total_count,
                    total_credit: MinorUnit::from_optional_bigdecimal_sum(total_credit),
                    total_debit: MinorUnit::from_optional_bigdecimal_sum(total_debit),
                    missing_count,
                    missing_credit: MinorUnit::from_optional_bigdecimal_sum(missing_credit),
                    missing_debit: MinorUnit::from_optional_bigdecimal_sum(missing_debit),
                },
            )
            .collect())
    }

    pub async fn aggregate_status_counts_by_rule_and_account(
        conn: &PgPooledConn,
        profile_id: &str,
        time_range: TimeRange,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<RuleAccountTransactionOverviewRow>> {
        let mut query = transaction::table
            .inner_join(
                entry::table.on(dsl::transaction_id
                    .eq(entry::logical_transaction_id)
                    .and(dsl::version.eq(entry::transaction_version))),
            )
            .group_by((
                dsl::rule_id,
                entry::account_id,
                dsl::status,
                entry::currency,
            ))
            .select((
                dsl::rule_id,
                entry::account_id,
                dsl::status,
                entry::currency,
                count(case_when(
                    entry::balance_direction.eq(BalanceDirection::Credit),
                    dsl::id,
                ))
                .aggregate_distinct(),
                count(case_when(
                    entry::balance_direction.eq(BalanceDirection::Debit),
                    dsl::id,
                ))
                .aggregate_distinct(),
                diesel::dsl::sum(case_when(
                    entry::balance_direction.eq(BalanceDirection::Credit),
                    entry::amount,
                )),
                diesel::dsl::sum(case_when(
                    entry::balance_direction.eq(BalanceDirection::Debit),
                    entry::amount,
                )),
                count(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map)
                        .and(entry::balance_direction.eq(BalanceDirection::Credit)),
                    dsl::id,
                ))
                .aggregate_distinct(),
                count(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map)
                        .and(entry::balance_direction.eq(BalanceDirection::Debit)),
                    dsl::id,
                ))
                .aggregate_distinct(),
                diesel::dsl::sum(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map)
                        .and(entry::balance_direction.eq(BalanceDirection::Credit)),
                    entry::amount,
                )),
                diesel::dsl::sum(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map)
                        .and(entry::balance_direction.eq(BalanceDirection::Debit)),
                    entry::amount,
                )),
            ))
            .into_boxed()
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::status.ne(DieselTransactionStatus::Archived))
            .filter(dsl::effective_at.ge(time_range.start_time));

        if let Some(end_time) = time_range.end_time {
            query = query.filter(dsl::effective_at.le(end_time));
        }

        if let Some(rule_ids) = rule_ids {
            query = query.filter(dsl::rule_id.eq_any(rule_ids));
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let rows: Vec<RuleAccountTransactionOverviewSummaryRowTuple> =
            generics::db_metrics::track_database_call::<entry::table, _, _>(
                query.get_results_async(conn),
                generics::db_metrics::DatabaseOperation::Filter,
            )
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| {
                format!(
                    "Error while aggregating transaction status counts by rule and account for profile_id: {profile_id}"
                )
            })?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    rule_id,
                    account_id,
                    status,
                    currency,
                    credit_count,
                    debit_count,
                    total_credit,
                    total_debit,
                    missing_credit_count,
                    missing_debit_count,
                    missing_credit,
                    missing_debit,
                )| {
                    RuleAccountTransactionOverviewRow {
                        rule_id,
                        account_id,
                        status,
                        currency,
                        credit_count,
                        debit_count,
                        total_credit: MinorUnit::from_optional_bigdecimal_sum(total_credit),
                        total_debit: MinorUnit::from_optional_bigdecimal_sum(total_debit),
                        missing_credit_count,
                        missing_debit_count,
                        missing_credit: MinorUnit::from_optional_bigdecimal_sum(missing_credit),
                        missing_debit: MinorUnit::from_optional_bigdecimal_sum(missing_debit),
                    }
                },
            )
            .collect())
    }

    pub async fn list_v2_paginated(
        conn: &PgPooledConn,
        profile_id: &str,
        filters: TransactionListFiltersV2,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        pagination: PaginationQuery<TransactionListSortField>,
    ) -> StorageResult<TransactionRuleViewPage> {
        let mut query = transaction::table
            .inner_join(recon_rule::table.on(recon_rule::id.eq(dsl::rule_id)))
            .select((transaction::all_columns, recon_rule::all_columns))
            .into_boxed();

        // Apply standard transaction-level filters
        if let Some(statuses) = filters.status {
            let has_expected = statuses.iter().any(|status| status.is_expected());
            let has_missing = statuses.iter().any(|status| status.is_missing());
            let include_expected_status_filter = has_expected && has_missing;
            let diesel_statuses: Vec<_> = statuses
                .iter()
                .map(TransactionListStatusFilter::to_diesel_status)
                .filter(|status| {
                    include_expected_status_filter
                        || !matches!(status, DieselTransactionStatus::Expected)
                })
                .collect();

            query = query.filter(dsl::status.eq_any(diesel_statuses));

            if has_expected != has_missing {
                for (rule_id, missing_before) in rule_missing_threshold_map {
                    if has_missing {
                        if let Some(missing_before) = missing_before {
                            query = query.or_filter(
                                dsl::status
                                    .eq(DieselTransactionStatus::Expected)
                                    .and(dsl::rule_id.eq(rule_id))
                                    .and(dsl::effective_at.lt(missing_before)),
                            );
                        }
                    } else if let Some(missing_before) = missing_before {
                        query = query.or_filter(
                            dsl::status
                                .eq(DieselTransactionStatus::Expected)
                                .and(dsl::rule_id.eq(rule_id))
                                .and(dsl::effective_at.ge(missing_before)),
                        );
                    } else {
                        query = query.or_filter(
                            dsl::status
                                .eq(DieselTransactionStatus::Expected)
                                .and(dsl::rule_id.eq(rule_id)),
                        );
                    }
                }
            }
        }

        query = query
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::status.ne(DieselTransactionStatus::Archived));

        if let Some(transaction_id) = filters.transaction_id {
            query = query.filter(dsl::transaction_id.eq(transaction_id));
        }

        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::effective_at.ge(time_range.start_time));
            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::effective_at.le(end_time));
            }
        }

        if let Some(rule_id) = filters.rule_id {
            query = query.filter(dsl::rule_id.eq(rule_id));
        }

        if let Some(order_id) = filters.entry_fields.order_id {
            query = query.filter(exists(
                entry::table
                    .filter(entry::logical_transaction_id.eq(dsl::transaction_id))
                    .filter(entry::transaction_version.eq(dsl::version))
                    .filter(entry::entry_identifier.eq(order_id))
                    .select(entry::id),
            ));
        }

        for metadata_filter in filters.entry_fields.metadata {
            query = query.filter(exists(
                search_entry_key::table
                    .filter(search_entry_key::profile_id.eq(profile_id.to_owned()))
                    .filter(search_entry_key::primary_transaction_id.eq(dsl::id))
                    .filter(search_entry_key::metadata_key.eq(metadata_filter.key))
                    .filter(search_entry_key::metadata_value.eq(Some(metadata_filter.value)))
                    .select(search_entry_key::id),
            ));
        }

        let ascending = pagination.ascending();

        query = match &pagination.sort_by {
            TransactionListSortField::Id { id } => {
                if let Some(cursor_id) = id {
                    query = if ascending {
                        query.filter(dsl::id.gt(cursor_id.clone()))
                    } else {
                        query.filter(dsl::id.lt(cursor_id.clone()))
                    };
                }
                if ascending {
                    query.order(dsl::id.asc())
                } else {
                    query.order(dsl::id.desc())
                }
            }
            TransactionListSortField::EffectiveAt { cursor_value } => {
                if let Some(cursor) = cursor_value {
                    query = if ascending {
                        query.filter(
                            dsl::effective_at
                                .gt(cursor.effective_at)
                                .or(dsl::effective_at
                                    .eq(cursor.effective_at)
                                    .and(dsl::id.gt(cursor.id.clone()))),
                        )
                    } else {
                        query.filter(
                            dsl::effective_at
                                .lt(cursor.effective_at)
                                .or(dsl::effective_at
                                    .eq(cursor.effective_at)
                                    .and(dsl::id.lt(cursor.id.clone()))),
                        )
                    };
                }
                if ascending {
                    query.order((dsl::effective_at.asc(), dsl::id.asc()))
                } else {
                    query.order((dsl::effective_at.desc(), dsl::id.desc()))
                }
            }
        };

        query = query.limit(i64::from(pagination.fetch_limit + 1));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let mut page_rows: Vec<(Self, ReconRule)> =
            generics::db_metrics::track_database_call::<transaction::table, _, _>(
                query.get_results_async(conn),
                generics::db_metrics::DatabaseOperation::Filter,
            )
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| {
                format!("Error while listing v2 transactions for profile_id: {profile_id}")
            })?;

        let request_limit = usize::from(pagination.fetch_limit);
        let has_more = page_rows.len() > request_limit;
        if has_more {
            page_rows.truncate(request_limit);
        }

        match pagination.direction {
            CursorDirection::Previous => page_rows.reverse(),
            CursorDirection::Next => {}
        }

        let views = page_rows
            .into_iter()
            .map(|(transaction, recon_rule)| TransactionRuleView {
                transaction,
                recon_rule,
            })
            .collect();

        Ok(TransactionRuleViewPage { views, has_more })
    }

    pub async fn aggregate_time_series_status_counts_by_rule(
        conn: &PgPooledConn,
        profile_id: &str,
        time_range: DefinedTimeRange,
        granularity: Granularity,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<TransactionTimeBucketRow>> {
        let bucket = date_trunc(granularity.to_string(), dsl::effective_at);

        let partition = (dsl::rule_id, bucket.clone(), dsl::status, dsl::currency);
        let mut query = transaction::table
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::status.ne(DieselTransactionStatus::Archived))
            .filter(dsl::effective_at.ge(time_range.start_time))
            .filter(dsl::effective_at.lt(time_range.end_time))
            .select((
                dsl::rule_id,
                bucket.clone(),
                dsl::status,
                dsl::currency,
                count(dsl::id).partition_by(partition.clone()),
                diesel::dsl::sum(dsl::credit_amount).partition_by(partition.clone()),
                diesel::dsl::sum(dsl::debit_amount).partition_by(partition.clone()),
                count(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map),
                    dsl::id,
                ))
                .partition_by(partition.clone()),
                diesel::dsl::sum(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map),
                    dsl::credit_amount,
                ))
                .partition_by(partition.clone()),
                diesel::dsl::sum(case_when(
                    transaction_is_missing_expression(&rule_missing_threshold_map),
                    dsl::debit_amount,
                ))
                .partition_by(partition),
            ))
            .into_boxed();

        if let Some(rule_ids) = rule_ids {
            query = query.filter(dsl::rule_id.eq_any(rule_ids));
        }

        let query = query.distinct();

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let rows: Vec<TransactionOverviewRowTuple> =
            generics::db_metrics::track_database_call::<transaction::table, _, _>(
                query.get_results_async(conn),
                generics::db_metrics::DatabaseOperation::Filter,
            )
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| {
                format!(
                    "Error while aggregating transaction time series for profile_id: {profile_id}"
                )
            })?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    rule_id,
                    bucket_start,
                    status,
                    currency,
                    total_count,
                    total_credit,
                    total_debit,
                    missing_count,
                    missing_credit,
                    missing_debit,
                )| {
                    TransactionTimeBucketRow {
                        bucket_start,
                        row: TransactionOverviewRow {
                            rule_id,
                            status,
                            currency,
                            total_count,
                            total_credit: MinorUnit::from_optional_bigdecimal_sum(total_credit),
                            total_debit: MinorUnit::from_optional_bigdecimal_sum(total_debit),
                            missing_count,
                            missing_credit: MinorUnit::from_optional_bigdecimal_sum(missing_credit),
                            missing_debit: MinorUnit::from_optional_bigdecimal_sum(missing_debit),
                        },
                    }
                },
            )
            .collect())
    }
}

impl TransactionNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Transaction> {
        generics::generic_insert(conn, self).await
    }
}
