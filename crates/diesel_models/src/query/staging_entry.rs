use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use bigdecimal::BigDecimal;
use common_enums::{BalanceDirection, Currency, DieselStagingEntryStatus};
use common_types::{
    date_time::TimeRange,
    pagination::{CursorDirection, PaginationQuery},
    staging_entry::{
        ListStagingEntriesFilters, StagingEntryListFiltersV2, StagingEntryListSortField,
    },
    units::MinorUnit,
};
use common_utils::{chunking::chunked, logging};
use diesel::dsl::{case_when, count_star};
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::{report, ResultExt};

const STAGING_ENTRY_BATCH_CHUNK_SIZE: usize = 1000;

use crate::{
    account::Account,
    errors,
    query::generics,
    schema::{
        account,
        staging_entry::{self, dsl},
        transformation_config,
    },
    staging_entry::{
        StagingEntry, StagingEntryAccountView, StagingEntryAccountViewPage, StagingEntryNew,
        StagingEntryOverviewRow, StagingEntryUpdateInternal,
    },
    transformation_config::TransformationConfig,
    PgPooledConn, StorageResult,
};

type StagingEntryOverviewSummaryRowTuple = (
    String,
    DieselStagingEntryStatus,
    Currency,
    i64,
    Option<BigDecimal>,
    Option<BigDecimal>,
);

impl StagingEntry {
    pub async fn find_by_id(conn: &PgPooledConn, id: String) -> StorageResult<Self> {
        generics::generic_find_one::<staging_entry::table, _, _>(conn, dsl::id.eq(id)).await
    }

    pub async fn find_by_id_and_profile(
        conn: &PgPooledConn,
        id: String,
        profile_id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<staging_entry::table, _, _>(
            conn,
            dsl::id.eq(id).and(dsl::profile_id.eq(profile_id)),
        )
        .await
    }

    pub async fn list_staging_entries_by_primary_ids_and_profile(
        conn: &PgPooledConn,
        ids: &[String],
        profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::staging_entry
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::id.eq_any(ids.to_vec()))
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<staging_entry::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while finding staging entries for profile_id: {profile_id} and ids: {ids:?}"
            )
        })
    }

    pub async fn list_with_account_by_profile_id(
        conn: &PgPooledConn,
        profile_id: &str,
        filters: ListStagingEntriesFilters,
        limit: Option<i64>,
    ) -> StorageResult<Vec<StagingEntryAccountView>> {
        let mut query = dsl::staging_entry
            .inner_join(account::table.on(dsl::account_id.eq(account::id)))
            .inner_join(
                transformation_config::table
                    .on(dsl::transformation_config_id.eq(transformation_config::id)),
            )
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(transformation_config::profile_id.eq(profile_id.to_owned()))
            .into_boxed();

        if let Some(status) = filters.status {
            query = query.filter(dsl::status.eq_any(status));
        }

        if let Some(entry_type) = filters.entry_type {
            query = query.filter(dsl::balance_direction.eq_any(entry_type));
        }

        if let Some(account_id) = filters.account_id {
            query = query.filter(dsl::account_id.eq(account_id));
        }

        if let Some(currency) = filters.currency {
            query = query.filter(dsl::currency.eq_any(currency));
        }

        if let Some(transformation_history_id) = filters.transformation_history_id {
            query = query.filter(dsl::transformation_history_id.eq(transformation_history_id));
        }

        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::effective_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::effective_at.le(end_time));
            }
        }

        if let Some(staging_entry_id) = filters.staging_entry_id {
            query = query.filter(dsl::staging_entry_id.eq(staging_entry_id));
        }

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        query
            .get_results_async::<(Self, Account, TransformationConfig)>(conn)
            .await
            .map(|results| {
                results
                    .into_iter()
                    .map(|(staging_entry, account, transformation_config)| {
                        StagingEntryAccountView {
                            staging_entry,
                            account,
                            transformation_config,
                        }
                    })
                    .collect()
            })
            .map_err(|err| {
                report!(errors::DatabaseError::Others).attach_printable(format!(
                    "Error while listing staging entries for profile_id: {profile_id}: {err}"
                ))
            })
    }

    pub async fn list_v2_paginated(
        conn: &PgPooledConn,
        profile_id: &str,
        filters: StagingEntryListFiltersV2,
        pagination: PaginationQuery<StagingEntryListSortField>,
    ) -> StorageResult<StagingEntryAccountViewPage> {
        let mut query = dsl::staging_entry
            .inner_join(account::table.on(dsl::account_id.eq(account::id)))
            .inner_join(
                transformation_config::table
                    .on(dsl::transformation_config_id.eq(transformation_config::id)),
            )
            .into_boxed();

        query = query
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(transformation_config::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::status.ne(DieselStagingEntryStatus::Archived));

        if let Some(status) = filters.status {
            query = query.filter(dsl::status.eq_any(status));
        }

        if let Some(entry_type) = filters.entry_type {
            query = query.filter(dsl::balance_direction.eq_any(entry_type));
        }

        if let Some(account_ids) = filters.account_ids {
            query = query.filter(dsl::account_id.eq_any(account_ids));
        }

        if let Some(currency) = filters.currency {
            query = query.filter(dsl::currency.eq_any(currency));
        }

        if let Some(transformation_config_ids) = filters.transformation_config_ids {
            query = query.filter(dsl::transformation_config_id.eq_any(transformation_config_ids));
        }

        if let Some(transformation_history_id) = filters.transformation_history_id {
            query = query.filter(dsl::transformation_history_id.eq(transformation_history_id));
        }

        if let Some(transformation_history_ids) = filters.transformation_history_ids {
            query = query.filter(dsl::transformation_history_id.eq_any(transformation_history_ids));
        }

        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::effective_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::effective_at.le(end_time));
            }
        }

        if let Some(staging_entry_id) = filters.staging_entry_id {
            query = query.filter(dsl::staging_entry_id.eq(staging_entry_id));
        }

        if let Some(order_id) = filters.order_id {
            query = query.filter(dsl::entry_identifier.eq(order_id));
        }

        let ascending = pagination.ascending();

        query = match &pagination.sort_by {
            StagingEntryListSortField::Id { id } => {
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
            StagingEntryListSortField::EffectiveAt { cursor_value } => {
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

        let mut views: Vec<StagingEntryAccountView> =
            generics::db_metrics::track_database_call::<staging_entry::table, _, _>(
                query.get_results_async::<(Self, Account, TransformationConfig)>(conn),
                generics::db_metrics::DatabaseOperation::Filter,
            )
            .await
            .map(|results| {
                results
                    .into_iter()
                    .map(|(staging_entry, account, transformation_config)| {
                        StagingEntryAccountView {
                            staging_entry,
                            account,
                            transformation_config,
                        }
                    })
                    .collect()
            })
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| {
                format!("Error while listing v2 staging entries for profile_id: {profile_id}")
            })?;

        let request_limit = usize::from(pagination.fetch_limit);
        let has_more = views.len() > request_limit;
        if has_more {
            views.truncate(request_limit);
        }

        if matches!(pagination.direction, CursorDirection::Previous) {
            views.reverse();
        }

        Ok(StagingEntryAccountViewPage { views, has_more })
    }

    pub async fn list_all_staging_entries(
        conn: &PgPooledConn,
        status: Option<DieselStagingEntryStatus>,
        limit: u16,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::staging_entry
            .order(dsl::created_at.asc())
            .limit(limit.into())
            .into_boxed();

        if let Some(status_filter) = status {
            query = query.filter(dsl::status.eq(status_filter));
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<staging_entry::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| "Error while listing all staging entries".to_string())
    }

    pub async fn count_pending_by_profile(
        conn: &PgPooledConn,
        profile_id: &str,
    ) -> StorageResult<i64> {
        let query = dsl::staging_entry
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::status.eq(DieselStagingEntryStatus::Pending))
            .count();

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<staging_entry::table, _, _>(
            query.get_result_async::<i64>(conn),
            generics::db_metrics::DatabaseOperation::Count,
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Error while counting pending staging entries for profile_id: {profile_id}")
        })
    }

    pub async fn update_by_id(
        conn: &PgPooledConn,
        id: &str,
        staging_entry_update: StagingEntryUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<staging_entry::table, _, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
            staging_entry_update,
        )
        .await
        .attach_printable_lazy(|| format!("Error while updating staging entry for id: {id}"))
    }

    pub async fn aggregate_status_counts_by_account(
        conn: &PgPooledConn,
        profile_id: &str,
        time_range: TimeRange,
        account_ids: Option<Vec<String>>,
        transformation_history_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<StagingEntryOverviewRow>> {
        let mut query = staging_entry::table
            .group_by((dsl::account_id, dsl::status, dsl::currency))
            .select((
                dsl::account_id,
                dsl::status,
                dsl::currency,
                count_star(),
                diesel::dsl::sum(case_when(
                    dsl::balance_direction.eq(BalanceDirection::Credit),
                    dsl::amount,
                )),
                diesel::dsl::sum(case_when(
                    dsl::balance_direction.eq(BalanceDirection::Debit),
                    dsl::amount,
                )),
            ))
            .into_boxed()
            .filter(dsl::profile_id.eq(profile_id.to_owned()))
            .filter(dsl::status.ne(DieselStagingEntryStatus::Archived))
            .filter(dsl::effective_at.ge(time_range.start_time));

        if let Some(end_time) = time_range.end_time {
            query = query.filter(dsl::effective_at.le(end_time));
        }

        if let Some(account_ids) = account_ids {
            query = query.filter(dsl::account_id.eq_any(account_ids));
        }

        if let Some(transformation_history_ids) = transformation_history_ids {
            query = query.filter(dsl::transformation_history_id.eq_any(transformation_history_ids));
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let rows: Vec<StagingEntryOverviewSummaryRowTuple> =
            generics::db_metrics::track_database_call::<staging_entry::table, _, _>(
                query.get_results_async(conn),
                generics::db_metrics::DatabaseOperation::Filter,
            )
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable_lazy(|| {
                format!(
                    "Error while aggregating staging entry status counts for profile_id: {profile_id}"
                )
            })?;

        Ok(rows
            .into_iter()
            .map(
                |(account_id, status, currency, count, credit_sum, debit_sum)| {
                    StagingEntryOverviewRow {
                        account_id,
                        status,
                        currency,
                        count,
                        credit_sum: MinorUnit::from_optional_bigdecimal_sum(credit_sum),
                        debit_sum: MinorUnit::from_optional_bigdecimal_sum(debit_sum),
                    }
                },
            )
            .collect())
    }
}

impl StagingEntryNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<StagingEntry> {
        generics::generic_insert(conn, self).await
    }

    pub async fn batch_insert_chunked(
        conn: &PgPooledConn,
        staging_entries: Vec<Self>,
    ) -> StorageResult<Vec<StagingEntry>> {
        let entries_count = staging_entries.len();

        conn.transaction_async(|conn| async move {
            let mut inserted: Vec<StagingEntry> = Vec::with_capacity(entries_count);

            for (chunk_idx, chunk) in
                chunked(staging_entries, STAGING_ENTRY_BATCH_CHUNK_SIZE).enumerate()
            {
                let chunk_len = chunk.len();
                let query = diesel::insert_into(staging_entry::table).values(chunk);
                logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

                let rows: Vec<StagingEntry> =
                    generics::db_metrics::track_database_call::<staging_entry::table, _, _>(
                        query.get_results_async(&conn),
                        generics::db_metrics::DatabaseOperation::Insert,
                    )
                    .await
                    .map_err(|e| {
                        logging::error!(
                            "Error while inserting staging-entry chunk \
                             (chunk_idx={chunk_idx}, chunk_len={chunk_len}, total={entries_count}): {e:?}",
                        );
                        errors::DatabaseError::Others
                    })?;

                inserted.extend(rows);
            }

            Result::<_, errors::DatabaseError>::Ok(inserted)
        })
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Transaction failed while batch inserting {entries_count} staging entries")
        })
    }
}
