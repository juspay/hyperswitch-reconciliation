use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use common_utils::{chunking::chunked, date_time, logging};
use diesel::{debug_query, pg::Pg, prelude::*};
use error_stack::{report, ResultExt};
use futures::future;

use crate::{
    entry::{Entry, EntryNew, EntryPage, EntryUpdateInternal, ExpectedEntryUpdateInternal},
    errors::DatabaseError,
    query::generics,
    schema::{
        account::{self},
        entry::{self, dsl},
    },
    search_duplicate::{SearchDuplicate, SearchDuplicateUpdate},
    PgPooledConn, StorageResult,
};

use common_enums::{BalanceDirection, EntryStatus, SearchDuplicateStatus};
use common_types::{
    entry::{EntryListFilters, EntryListSortField, ListEntriesFilters},
    pagination::{CursorDirection, PaginationQuery},
    units::MinorUnit,
};

const ENTRY_BATCH_CHUNK_SIZE: usize = 1000;

/// Represents balance changes for an account
#[derive(Debug, Clone, PartialEq)]
pub struct BalanceDelta {
    pub pending_credits: i64,
    pub pending_debits: i64,
    pub matched_credits: i64,
    pub matched_debits: i64,
    pub expected_credits: i64,
    pub expected_debits: i64,
    pub mismatched_credits: i64,
    pub mismatched_debits: i64,
    pub posted_credits: i64,
    pub posted_debits: i64,
}

impl BalanceDelta {
    pub fn negative(self) -> Self {
        Self {
            pending_credits: -self.pending_credits,
            pending_debits: -self.pending_debits,
            matched_credits: -self.matched_credits,
            matched_debits: -self.matched_debits,
            expected_credits: -self.expected_credits,
            expected_debits: -self.expected_debits,
            mismatched_credits: -self.mismatched_credits,
            mismatched_debits: -self.mismatched_debits,
            posted_credits: -self.posted_credits,
            posted_debits: -self.posted_debits,
        }
    }
}

/// Enum representing different balance update intentions
#[derive(Debug, Clone)]
pub enum BalanceEffect {
    ApplyEntry {
        entry_type: BalanceDirection,
        status: EntryStatus,
        amount: MinorUnit,
    },
    ArchiveEntry {
        entry_type: BalanceDirection,
        original_status: EntryStatus,
        amount: MinorUnit,
    },
}

/// Calculate balance deltas for a given entry status and type
pub fn calculate_balance_deltas(
    amount: i64,
    entry_type: BalanceDirection,
    status: EntryStatus,
) -> BalanceDelta {
    match (status, entry_type) {
        // Posted entries only affect posted balances
        (EntryStatus::Posted, BalanceDirection::Credit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: amount,
            posted_debits: 0,
        },
        (EntryStatus::Posted, BalanceDirection::Debit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: amount,
        },
        // Pending entries only affect pending balances
        (EntryStatus::Pending, BalanceDirection::Credit) => BalanceDelta {
            pending_credits: amount,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
        (EntryStatus::Pending, BalanceDirection::Debit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: amount,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
        // Expected entries only affect expected balances
        (EntryStatus::Expected, BalanceDirection::Credit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: amount,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
        (EntryStatus::Expected, BalanceDirection::Debit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: amount,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
        // Mismatched entries only affect mismatched balances
        (EntryStatus::Mismatched, BalanceDirection::Credit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: amount,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
        (EntryStatus::Mismatched, BalanceDirection::Debit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: amount,
            posted_credits: 0,
            posted_debits: 0,
        },
        // Archived entries don't affect any balance
        (EntryStatus::Archived, _) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
        (EntryStatus::Void, _) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
        (EntryStatus::Matched, BalanceDirection::Credit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: amount,
            matched_debits: 0,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
        (EntryStatus::Matched, BalanceDirection::Debit) => BalanceDelta {
            pending_credits: 0,
            pending_debits: 0,
            matched_credits: 0,
            matched_debits: amount,
            expected_credits: 0,
            expected_debits: 0,
            mismatched_credits: 0,
            mismatched_debits: 0,
            posted_credits: 0,
            posted_debits: 0,
        },
    }
}

/// Unified function to update account balance based on the specified effect
async fn update_account_balance(
    conn: &PgPooledConn,
    account_id: &str,
    effect: BalanceEffect,
) -> StorageResult<()> {
    let account_filter = account::table.filter(account::dsl::id.eq(account_id.to_owned()));

    // Calculate deltas based on the effect
    let deltas = match effect {
        BalanceEffect::ApplyEntry {
            entry_type,
            status,
            amount,
        } => {
            let amount_i64 = amount.get_amount_as_i64();
            calculate_balance_deltas(amount_i64, entry_type, status)
        }
        BalanceEffect::ArchiveEntry {
            entry_type,
            original_status,
            amount,
        } => {
            let amount_i64 = amount.get_amount_as_i64();
            calculate_balance_deltas(amount_i64, entry_type, original_status).negative()
        }
    };

    generics::db_metrics::track_database_call::<account::table, _, _>(
        diesel::update(account_filter)
            .set((
                account::dsl::pending_credits
                    .eq(account::dsl::pending_credits + deltas.pending_credits),
                account::dsl::pending_debits
                    .eq(account::dsl::pending_debits + deltas.pending_debits),
                account::dsl::posted_credits
                    .eq(account::dsl::posted_credits + deltas.posted_credits),
                account::dsl::posted_debits.eq(account::dsl::posted_debits + deltas.posted_debits),
                account::dsl::expected_credits
                    .eq(account::dsl::expected_credits + deltas.expected_credits),
                account::dsl::expected_debits
                    .eq(account::dsl::expected_debits + deltas.expected_debits),
                account::dsl::mismatched_credits
                    .eq(account::dsl::mismatched_credits + deltas.mismatched_credits),
                account::dsl::mismatched_debits
                    .eq(account::dsl::mismatched_debits + deltas.mismatched_debits),
                account::dsl::matched_credits
                    .eq(account::dsl::matched_credits + deltas.matched_credits),
                account::dsl::matched_debits
                    .eq(account::dsl::matched_debits + deltas.matched_debits),
                account::dsl::last_modified_at.eq(date_time::now()),
            ))
            .execute_async(conn),
        generics::db_metrics::DatabaseOperation::Update,
    )
    .await
    .change_context(DatabaseError::Others)
    .attach_printable("Error updating account balances")?;

    Ok(())
}

impl EntryNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Entry> {
        let account_id = self.account_id.clone();
        let amount = self.amount;
        let entry_type = self.balance_direction;
        let status = self.status;

        conn.transaction_async(|conn| async move {
            // First, insert the entry
            let inserted_entry: Entry =
                generics::generic_insert(&conn, self).await.map_err(|e| {
                    logging::error!("Error inserting entry: {:?}", e);
                    *e.current_context()
                })?;

            // Update account balance
            update_account_balance(
                &conn,
                &account_id,
                BalanceEffect::ApplyEntry {
                    entry_type,
                    status,
                    amount,
                },
            )
            .await
            .map_err(|e| {
                logging::error!("Error updating account balance for new entry: {:?}", e);
                *e.current_context()
            })?;

            if let Some(staging_entry_id) = inserted_entry.primary_staging_entry_id.clone() {
                let status = match inserted_entry.status {
                    EntryStatus::Void | EntryStatus::Archived => SearchDuplicateStatus::Archived,
                    EntryStatus::Expected
                    | EntryStatus::Posted
                    | EntryStatus::Matched
                    | EntryStatus::Mismatched
                    | EntryStatus::Pending => SearchDuplicateStatus::Active,
                };

                SearchDuplicate::update_by_primary_staging_entry_id(
                    &conn,
                    &staging_entry_id,
                    SearchDuplicateUpdate::StatusUpdate { status }.into(),
                )
                .await
                .map_err(|e| {
                    logging::error!(
                        "Error updating search duplicate status for new entry: {:?}",
                        e
                    );
                    *e.current_context()
                })?;
            }

            Result::<_, DatabaseError>::Ok(inserted_entry)
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable("Transaction failed while inserting entry and updating account balance")
    }

    pub async fn batch_insert_chunked(
        conn: &PgPooledConn,
        entries: Vec<Self>,
    ) -> StorageResult<Vec<Entry>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let entry_count = entries.len();

        conn.transaction_async(|conn| async move {
            let mut inserted_entries: Vec<Entry> = Vec::with_capacity(entry_count);

            for (chunk_idx, chunk) in chunked(entries.clone(), ENTRY_BATCH_CHUNK_SIZE).enumerate() {
                let chunk_len = chunk.len();
                let query = diesel::insert_into(entry::table).values(chunk);
                logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

                let rows: Vec<Entry> =
                    generics::db_metrics::track_database_call::<entry::table, _, _>(
                        query
                            .get_results_async(&conn),
                        generics::db_metrics::DatabaseOperation::Insert,
                    )
                    .await
                    .map_err(|e| {
                        logging::error!(
                            "Error while inserting entry chunk (chunk_idx={chunk_idx}, chunk_len={chunk_len}, total={entry_count}): {e:?}",
                        );
                        DatabaseError::Others
                    })?;

                inserted_entries.extend(rows);
            }

            // Update account balance for each inserted entry using parallel processing
            future::try_join_all(
                entries
                    .iter()
                    .map(|entry| {
                        update_account_balance(
                            &conn,
                            &entry.account_id,
                            BalanceEffect::ApplyEntry {
                                entry_type: entry.balance_direction,
                                status: entry.status,
                                amount: entry.amount,
                            },
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .await
            .map_err(|e| {
                logging::error!("Error updating account balances for batch insert: {:?}", e);
                *e.current_context()
            })?;

            // Reactivate search duplicates
            future::try_join_all(entries.iter().filter_map(|entry| {
                entry
                    .primary_staging_entry_id
                    .as_ref()
                    .map(|staging_entry_id| {
                        let status = match entry.status {
                            EntryStatus::Void | EntryStatus::Archived => SearchDuplicateStatus::Archived,
                            EntryStatus::Expected | EntryStatus::Posted | EntryStatus::Matched | EntryStatus::Mismatched | EntryStatus::Pending => SearchDuplicateStatus::Active
                        };
                        SearchDuplicate::update_by_primary_staging_entry_id(
                            &conn,
                            staging_entry_id,
                            SearchDuplicateUpdate::StatusUpdate {
                                status,
                            }.into(),
                        )
                    })
            }))
            .await
            .map_err(|e| {
                logging::error!("Error updating search duplicates for batch insert: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(inserted_entries)
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(move || {
            format!("Transaction failed while batch inserting {entry_count} entries and updating account balances")
        })
    }
}

impl Entry {
    pub async fn find_by_entry_id(conn: &PgPooledConn, entry_id: String) -> StorageResult<Self> {
        generics::generic_find_by_id::<entry::table, _, _>(conn, entry_id).await
    }

    pub async fn list_by_account_id(
        conn: &PgPooledConn,
        account_id: &str,
        filters: ListEntriesFilters,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::entry
            .filter(dsl::account_id.eq(account_id.to_owned()))
            .into_boxed();

        if let Some(status) = filters.status {
            query = query.filter(dsl::status.eq_any(status));
        }

        if let Some(entry_type) = filters.entry_type {
            query = query.filter(dsl::balance_direction.eq_any(entry_type));
        }

        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::effective_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::effective_at.le(end_time));
            }
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        match generics::db_metrics::track_database_call::<entry::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(err) => match err {
                diesel::result::Error::NotFound => {
                    Err(report!(err)).change_context(DatabaseError::NotFound)
                }
                _ => Err(report!(err)).change_context(DatabaseError::Others),
            },
        }
        .attach_printable_lazy(|| {
            format!("Error while listing entries for account_id: {account_id}")
        })
    }

    pub async fn list_by_transaction_id_and_version(
        conn: &PgPooledConn,
        transaction_id: &str,
        version: Option<i32>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = dsl::entry
            .filter(dsl::logical_transaction_id.eq(transaction_id.to_owned()))
            .into_boxed();

        if let Some(version) = version {
            query = query.filter(dsl::transaction_version.eq(version));
        }

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        match generics::db_metrics::track_database_call::<entry::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(err) => match err {
                diesel::result::Error::NotFound => {
                    Err(report!(err)).change_context(DatabaseError::NotFound)
                }
                _ => Err(report!(err)).change_context(DatabaseError::Others),
            },
        }
        .attach_printable_lazy(|| {
            format!("Error while listing entries for transaction_id: {transaction_id}")
        })
    }

    pub async fn list_paginated(
        conn: &PgPooledConn,
        filters: EntryListFilters,
        pagination: PaginationQuery<EntryListSortField>,
    ) -> StorageResult<EntryPage> {
        let logical_transaction_id = filters.logical_transaction_id;

        let mut query = dsl::entry
            .filter(dsl::logical_transaction_id.eq(logical_transaction_id.clone()))
            .filter(dsl::transaction_version.eq(filters.version))
            .into_boxed();

        if let Some(account_ids) = filters.account_ids {
            query = query.filter(dsl::account_id.eq_any(account_ids));
        }

        if let Some(entry_type) = filters.entry_type {
            query = query.filter(dsl::balance_direction.eq(entry_type));
        }

        if let Some(currency) = filters.currency {
            query = query.filter(dsl::currency.eq_any(currency));
        }

        if let Some(status) = filters.status {
            query = query.filter(dsl::status.eq_any(status));
        }

        if let Some(time_range) = filters.time_range {
            query = query.filter(dsl::effective_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                query = query.filter(dsl::effective_at.le(end_time));
            }
        }

        if let Some(staging_entry_ids) = filters.staging_entry_ids {
            query = query.filter(dsl::primary_staging_entry_id.eq_any(staging_entry_ids));
        }

        if let Some(transformation_config_ids) = filters.transformation_config_ids {
            query = query.filter(dsl::transformation_config_id.eq_any(transformation_config_ids));
        }

        if let Some(order_ids) = filters.order_ids {
            query = query.filter(dsl::entry_identifier.eq_any(order_ids));
        }

        let ascending = pagination.ascending();

        query = match &pagination.sort_by {
            EntryListSortField::Id { id } => {
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
            EntryListSortField::EffectiveAt { cursor_value } => {
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

        let mut entries: Vec<Self> =
            generics::db_metrics::track_database_call::<entry::table, _, _>(
                query.get_results_async(conn),
                generics::db_metrics::DatabaseOperation::Filter,
            )
            .await
            .change_context(DatabaseError::Others)
            .attach_printable_lazy(|| {
                format!(
                    "Error while listing entries for logical_transaction_id: {logical_transaction_id}"
                )
            })?;

        let request_limit = usize::from(pagination.fetch_limit);
        let has_more = entries.len() > request_limit;
        if has_more {
            entries.truncate(request_limit);
        }

        if matches!(pagination.direction, CursorDirection::Previous) {
            entries.reverse();
        }

        Ok(EntryPage { entries, has_more })
    }

    pub async fn list_expected_by_transaction_id_version_and_account(
        conn: &PgPooledConn,
        transaction_id: &str,
        version: i32,
        account_id: &str,
    ) -> StorageResult<Vec<Self>> {
        let query = dsl::entry
            .filter(dsl::logical_transaction_id.eq(transaction_id.to_owned()))
            .filter(dsl::transaction_version.eq(version))
            .filter(dsl::account_id.eq(account_id.to_owned()))
            .filter(dsl::status.eq(EntryStatus::Expected));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<entry::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while listing expected entries for transaction_id: {transaction_id}, version: {version}, account_id: {account_id}"
            )
        })
    }

    pub async fn find_non_expected_by_transaction_id_version_and_account(
        conn: &PgPooledConn,
        transaction_id: &str,
        version: i32,
        account_id: &str,
    ) -> StorageResult<Option<Self>> {
        let query = dsl::entry
            .filter(dsl::logical_transaction_id.eq(transaction_id.to_owned()))
            .filter(dsl::transaction_version.eq(version))
            .filter(dsl::account_id.eq(account_id.to_owned()))
            .filter(dsl::status.ne(EntryStatus::Expected))
            .limit(1);

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let entries: Vec<Self> = generics::db_metrics::track_database_call::<entry::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while finding non-expected entry for transaction_id: {transaction_id}, version: {version}, account_id: {account_id}"
            )
        })?;

        Ok(entries.into_iter().next())
    }

    pub async fn find_first_pending_source_by_transaction_id_and_version(
        conn: &PgPooledConn,
        transaction_id: &str,
        version: i32,
        source_account_id: &str,
    ) -> StorageResult<Self> {
        let query = dsl::entry
            .filter(dsl::logical_transaction_id.eq(transaction_id.to_owned()))
            .filter(dsl::transaction_version.eq(version))
            .filter(dsl::account_id.eq(source_account_id.to_owned()))
            .filter(dsl::status.eq(EntryStatus::Pending))
            .order((dsl::effective_at.asc(), dsl::id.asc()))
            .limit(1);

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let result: Self = generics::db_metrics::track_database_call::<entry::table, _, _>(
            query.get_result_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!(
                "Error while finding first pending source entry for transaction_id: {transaction_id}, version: {version}, source_account_id: {source_account_id}",
            )
        })?;

        Ok(result)
    }

    pub async fn update_expected_for_source_append(
        conn: &PgPooledConn,
        current: &Self,
        update: ExpectedEntryUpdateInternal,
    ) -> StorageResult<Self> {
        update_account_balance(
            conn,
            &current.account_id,
            BalanceEffect::ArchiveEntry {
                entry_type: current.balance_direction,
                original_status: EntryStatus::Expected,
                amount: current.amount,
            },
        )
        .await?;

        let updated =
            generics::generic_update_with_unique_predicate_get_result::<entry::table, _, _, Self>(
                conn,
                dsl::id
                    .eq(current.id.clone())
                    .and(dsl::status.eq(EntryStatus::Expected)),
                update,
            )
            .await
            .attach_printable_lazy(|| {
                format!(
                    "Error while updating expected entry after source append: {}",
                    current.id
                )
            })?;

        update_account_balance(
            conn,
            &updated.account_id,
            BalanceEffect::ApplyEntry {
                entry_type: updated.balance_direction,
                status: EntryStatus::Expected,
                amount: updated.amount,
            },
        )
        .await?;

        Ok(updated)
    }

    pub async fn discard_entry_by_id(
        conn: &PgPooledConn,
        entry_id: &str,
        entry_update: EntryUpdateInternal,
    ) -> StorageResult<Self> {
        conn.transaction_async(|conn| async move {
            let updated_entry = generics::generic_update_with_unique_predicate_get_result::<
                entry::table,
                _,
                _,
                Self,
            >(
                &conn,
                dsl::id
                    .eq(entry_id.to_owned())
                    .and(dsl::status.ne(EntryStatus::Archived)),
                entry_update,
            )
            .await
            .map_err(|e| {
                logging::error!(
                    "Error while discarding entry for entry_id: {}: {:?}",
                    entry_id,
                    e
                );
                *e.current_context()
            })?;

            if let Some(original_status) = updated_entry.discarded_status {
                update_account_balance(
                    &conn,
                    &updated_entry.account_id,
                    BalanceEffect::ArchiveEntry {
                        entry_type: updated_entry.balance_direction,
                        original_status,
                        amount: updated_entry.amount,
                    },
                )
                .await
                .map_err(|e| {
                    logging::error!(
                        "Error updating account balance for entry_id: {}: {:?}",
                        entry_id,
                        e
                    );
                    *e.current_context()
                })?;
            }

            Result::<_, DatabaseError>::Ok(updated_entry)
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Transaction failed while discarding entry for entry_id: {entry_id}")
        })
    }

    pub async fn batch_discard_entries_by_transaction_id(
        conn: &PgPooledConn,
        transaction_id: &str,
        entry_update: EntryUpdateInternal,
    ) -> StorageResult<Vec<Self>> {
        conn.transaction_async(|conn| async move {
            let updated_entries = generics::generic_update_with_results::<entry::table, _, _, Self>(
                &conn,
                dsl::logical_transaction_id
                    .eq(transaction_id.to_owned())
                    .and(dsl::status.ne(EntryStatus::Archived)),
                entry_update,
            )
            .await
            .map_err(|e| {
                logging::error!("Error while batch discarding entries for transaction_id: {}: {:?}", transaction_id, e);
                *e.current_context()
            })?;

            future::try_join_all(
                updated_entries
                    .iter()
                    .filter_map(|updated_entry| {
                        updated_entry.discarded_status.map(|original_status| {
                            update_account_balance(
                                &conn,
                                &updated_entry.account_id,
                                BalanceEffect::ArchiveEntry {
                                    entry_type: updated_entry.balance_direction,
                                    original_status,
                                    amount: updated_entry.amount,
                                },
                            )
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .await
            .map_err(|e| {
                logging::error!("Error updating account balances for batch discard: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(updated_entries)
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| {
            format!("Transaction failed while batch discarding entries for transaction_id: {transaction_id}")
        })
    }

    pub async fn list_entries_by_ids(
        conn: &PgPooledConn,
        entry_ids: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        if entry_ids.is_empty() {
            return Ok(Vec::new());
        }

        let query = dsl::entry.filter(dsl::id.eq_any(entry_ids.clone()));

        logging::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        generics::db_metrics::track_database_call::<entry::table, _, _>(
            query.get_results_async(conn),
            generics::db_metrics::DatabaseOperation::Filter,
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable_lazy(|| format!("Error while listing entries by IDs: {entry_ids:?}"))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(EntryStatus::Posted, BalanceDirection::Credit, 100, BalanceDelta { pending_credits: 0, pending_debits: 0, matched_credits: 0, matched_debits: 0, posted_credits: 100, posted_debits: 0, expected_credits: 0, expected_debits: 0, mismatched_credits: 0, mismatched_debits: 0 })]
    #[case(EntryStatus::Posted, BalanceDirection::Debit, 100, BalanceDelta { pending_credits: 0, pending_debits: 0, matched_credits: 0, matched_debits: 0, posted_credits: 0, posted_debits: 100, expected_credits: 0, expected_debits: 0, mismatched_credits: 0, mismatched_debits: 0 })]
    #[case(EntryStatus::Pending, BalanceDirection::Credit, 100, BalanceDelta { pending_credits: 100, pending_debits: 0,matched_credits: 0, matched_debits: 0, posted_credits: 0, posted_debits: 0, expected_credits: 0, expected_debits: 0, mismatched_credits: 0, mismatched_debits: 0 })]
    #[case(EntryStatus::Pending, BalanceDirection::Debit, 100, BalanceDelta { pending_credits: 0, pending_debits: 100,matched_credits: 0, matched_debits: 0, posted_credits: 0, posted_debits: 0, expected_credits: 0, expected_debits: 0, mismatched_credits: 0, mismatched_debits: 0 })]
    #[case(EntryStatus::Expected, BalanceDirection::Credit, 100, BalanceDelta { pending_credits: 0, pending_debits: 0,matched_credits: 0, matched_debits: 0, posted_credits: 0, posted_debits: 0, expected_credits: 100, expected_debits: 0, mismatched_credits: 0, mismatched_debits: 0 })]
    #[case(EntryStatus::Expected, BalanceDirection::Debit, 100, BalanceDelta { pending_credits: 0, pending_debits: 0,matched_credits: 0, matched_debits: 0, posted_credits: 0, posted_debits: 0, expected_credits: 0, expected_debits: 100, mismatched_credits: 0, mismatched_debits: 0 })]
    #[case(EntryStatus::Mismatched, BalanceDirection::Credit, 100, BalanceDelta { pending_credits: 0, pending_debits: 0,matched_credits: 0, matched_debits: 0, posted_credits: 0, posted_debits: 0, expected_credits: 0, expected_debits: 0, mismatched_credits: 100, mismatched_debits: 0 })]
    #[case(EntryStatus::Mismatched, BalanceDirection::Debit, 100, BalanceDelta { pending_credits: 0, pending_debits: 0,matched_credits: 0, matched_debits: 0, posted_credits: 0, posted_debits: 0, expected_credits: 0, expected_debits: 0, mismatched_credits: 0, mismatched_debits: 100 })]
    #[case(EntryStatus::Archived, BalanceDirection::Credit, 100, BalanceDelta { pending_credits: 0, pending_debits: 0,matched_credits: 0, matched_debits: 0, posted_credits: 0, posted_debits: 0, expected_credits: 0, expected_debits: 0, mismatched_credits: 0, mismatched_debits: 0 })]
    #[case(EntryStatus::Archived, BalanceDirection::Debit, 100, BalanceDelta { pending_credits: 0, pending_debits: 0, matched_credits: 0, matched_debits: 0,posted_credits: 0, posted_debits: 0, expected_credits: 0, expected_debits: 0, mismatched_credits: 0, mismatched_debits: 0 })]
    fn test_calculate_balance_deltas(
        #[case] status: EntryStatus,
        #[case] entry_type: BalanceDirection,
        #[case] amount: i64,
        #[case] expected: BalanceDelta,
    ) {
        let result = calculate_balance_deltas(amount, entry_type, status);
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(0)]
    #[case(1_000_000_000)] // 1 billion
    #[case(50)]
    fn test_calculate_balance_deltas_various_amounts(#[case] amount: i64) {
        let result =
            calculate_balance_deltas(amount, BalanceDirection::Credit, EntryStatus::Pending);
        assert_eq!(
            result,
            BalanceDelta {
                pending_credits: amount,
                pending_debits: 0,
                posted_credits: 0,
                posted_debits: 0,
                expected_credits: 0,
                expected_debits: 0,
                mismatched_credits: 0,
                mismatched_debits: 0,
                matched_credits: 0,
                matched_debits: 0,
            }
        );

        let result = calculate_balance_deltas(amount, BalanceDirection::Debit, EntryStatus::Posted);
        assert_eq!(
            result,
            BalanceDelta {
                pending_credits: 0,
                pending_debits: 0,
                posted_credits: 0,
                posted_debits: amount,
                expected_credits: 0,
                expected_debits: 0,
                mismatched_credits: 0,
                mismatched_debits: 0,
                matched_credits: 0,
                matched_debits: 0,
            }
        );
    }

    #[rstest]
    #[case(EntryStatus::Pending, EntryStatus::Posted, BalanceDirection::Credit, 100, (-100, 0, 100, 0))]
    #[case(EntryStatus::Pending, EntryStatus::Posted, BalanceDirection::Debit, 100, (0, -100, 0, 100))]
    #[case(EntryStatus::Posted, EntryStatus::Archived, BalanceDirection::Credit, 100, (0, 0, -100, 0))]
    #[case(EntryStatus::Posted, EntryStatus::Archived, BalanceDirection::Debit, 100, (0, 0, 0, -100))]
    #[case(EntryStatus::Pending, EntryStatus::Archived, BalanceDirection::Credit, 100, (-100, 0, 0, 0))]
    #[case(EntryStatus::Pending, EntryStatus::Archived, BalanceDirection::Debit, 100, (0, -100, 0, 0))]
    #[case(EntryStatus::Expected, EntryStatus::Pending, BalanceDirection::Credit, 100, (100, 0, 0, 0))]
    #[case(EntryStatus::Mismatched, EntryStatus::Expected, BalanceDirection::Debit, 100, (0, 0, 0, 0))]
    fn test_status_change_balance_deltas(
        #[case] old_status: EntryStatus,
        #[case] new_status: EntryStatus,
        #[case] entry_type: BalanceDirection,
        #[case] amount: i64,
        #[case] expected_net_change: (i64, i64, i64, i64),
    ) {
        let old_deltas = calculate_balance_deltas(amount, entry_type, old_status);
        let new_deltas = calculate_balance_deltas(amount, entry_type, new_status);

        let net_change = (
            new_deltas.pending_credits - old_deltas.pending_credits,
            new_deltas.pending_debits - old_deltas.pending_debits,
            new_deltas.posted_credits - old_deltas.posted_credits,
            new_deltas.posted_debits - old_deltas.posted_debits,
        );

        assert_eq!(net_change, expected_net_change);
    }

    #[test]
    fn test_balance_deltas_consistency() {
        // Test that the balance calculation logic is consistent
        let amount = 250;

        for status in [
            EntryStatus::Pending,
            EntryStatus::Posted,
            EntryStatus::Expected,
            EntryStatus::Mismatched,
            EntryStatus::Archived,
        ] {
            for entry_type in [BalanceDirection::Credit, BalanceDirection::Debit] {
                let deltas = calculate_balance_deltas(amount, entry_type, status);

                if status == EntryStatus::Archived {
                    assert_eq!(
                        deltas,
                        BalanceDelta {
                            pending_credits: 0,
                            pending_debits: 0,
                            posted_credits: 0,
                            posted_debits: 0,
                            expected_credits: 0,
                            expected_debits: 0,
                            mismatched_credits: 0,
                            mismatched_debits: 0,
                            matched_credits: 0,
                            matched_debits: 0,
                        }
                    );
                } else {
                    // Each status should only affect its specific balance type
                    match status {
                        EntryStatus::Pending => {
                            if entry_type == BalanceDirection::Credit {
                                assert_eq!(deltas.pending_credits, amount);
                                assert_eq!(deltas.pending_debits, 0);
                            } else {
                                assert_eq!(deltas.pending_credits, 0);
                                assert_eq!(deltas.pending_debits, amount);
                            }
                            assert_eq!(deltas.posted_credits, 0);
                            assert_eq!(deltas.posted_debits, 0);
                            assert_eq!(deltas.expected_credits, 0);
                            assert_eq!(deltas.expected_debits, 0);
                            assert_eq!(deltas.mismatched_credits, 0);
                            assert_eq!(deltas.mismatched_debits, 0);
                            assert_eq!(deltas.matched_credits, 0);
                            assert_eq!(deltas.matched_debits, 0);
                        }
                        EntryStatus::Posted => {
                            if entry_type == BalanceDirection::Credit {
                                assert_eq!(deltas.posted_credits, amount);
                                assert_eq!(deltas.posted_debits, 0);
                            } else {
                                assert_eq!(deltas.posted_credits, 0);
                                assert_eq!(deltas.posted_debits, amount);
                            }
                            assert_eq!(deltas.pending_credits, 0);
                            assert_eq!(deltas.pending_debits, 0);
                            assert_eq!(deltas.expected_credits, 0);
                            assert_eq!(deltas.expected_debits, 0);
                            assert_eq!(deltas.mismatched_credits, 0);
                            assert_eq!(deltas.mismatched_debits, 0);
                            assert_eq!(deltas.matched_credits, 0);
                            assert_eq!(deltas.matched_debits, 0);
                        }
                        EntryStatus::Matched => {
                            assert_eq!(deltas.posted_credits, 0);
                            assert_eq!(deltas.posted_debits, 0);
                            assert_eq!(deltas.pending_credits, 0);
                            assert_eq!(deltas.pending_debits, 0);
                            assert_eq!(deltas.expected_credits, 0);
                            assert_eq!(deltas.expected_debits, 0);
                            assert_eq!(deltas.mismatched_credits, 0);
                            assert_eq!(deltas.mismatched_debits, 0);
                            if entry_type == BalanceDirection::Credit {
                                assert_eq!(deltas.matched_credits, amount);
                                assert_eq!(deltas.matched_debits, 0);
                            } else {
                                assert_eq!(deltas.matched_credits, 0);
                                assert_eq!(deltas.matched_debits, amount);
                            }
                        }
                        EntryStatus::Expected => {
                            if entry_type == BalanceDirection::Credit {
                                assert_eq!(deltas.expected_credits, amount);
                                assert_eq!(deltas.expected_debits, 0);
                            } else {
                                assert_eq!(deltas.expected_credits, 0);
                                assert_eq!(deltas.expected_debits, amount);
                            }
                            assert_eq!(deltas.pending_credits, 0);
                            assert_eq!(deltas.pending_debits, 0);
                            assert_eq!(deltas.posted_credits, 0);
                            assert_eq!(deltas.posted_debits, 0);
                            assert_eq!(deltas.mismatched_credits, 0);
                            assert_eq!(deltas.mismatched_debits, 0);
                            assert_eq!(deltas.matched_credits, 0);
                            assert_eq!(deltas.matched_debits, 0);
                        }
                        EntryStatus::Mismatched => {
                            if entry_type == BalanceDirection::Credit {
                                assert_eq!(deltas.mismatched_credits, amount);
                                assert_eq!(deltas.mismatched_debits, 0);
                            } else {
                                assert_eq!(deltas.mismatched_credits, 0);
                                assert_eq!(deltas.mismatched_debits, amount);
                            }
                            assert_eq!(deltas.pending_credits, 0);
                            assert_eq!(deltas.pending_debits, 0);
                            assert_eq!(deltas.posted_credits, 0);
                            assert_eq!(deltas.posted_debits, 0);
                            assert_eq!(deltas.expected_credits, 0);
                            assert_eq!(deltas.expected_debits, 0);
                            assert_eq!(deltas.matched_credits, 0);
                            assert_eq!(deltas.matched_debits, 0);
                        }
                        EntryStatus::Archived => {
                            // Archived entries shouldn't affect any balances - this is tested above
                        }
                        EntryStatus::Void => {
                            // Void entries shouldn't affect any balances
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_balance_deltas_status_separation() {
        // Test that each status only affects its specific balance type
        let amount = 100;

        let posted_deltas =
            calculate_balance_deltas(amount, BalanceDirection::Credit, EntryStatus::Posted);
        assert_eq!(posted_deltas.posted_credits, amount);
        assert_eq!(posted_deltas.pending_credits, 0);
        assert_eq!(posted_deltas.expected_credits, 0);
        assert_eq!(posted_deltas.mismatched_credits, 0);

        let expected_deltas =
            calculate_balance_deltas(amount, BalanceDirection::Credit, EntryStatus::Expected);
        assert_eq!(expected_deltas.expected_credits, amount);
        assert_eq!(expected_deltas.pending_credits, 0);
        assert_eq!(expected_deltas.posted_credits, 0);
        assert_eq!(expected_deltas.mismatched_credits, 0);
    }

    #[test]
    fn test_complex_status_transitions() {
        // Test complex status transitions that might happen in real scenarios
        let amount = 500;
        let entry_type = BalanceDirection::Credit;

        // Scenario: Entry goes from Expected -> Pending -> Posted -> Archived
        let expected_deltas = calculate_balance_deltas(amount, entry_type, EntryStatus::Expected);
        let pending_deltas = calculate_balance_deltas(amount, entry_type, EntryStatus::Pending);
        let posted_deltas = calculate_balance_deltas(amount, entry_type, EntryStatus::Posted);
        let archived_deltas = calculate_balance_deltas(amount, entry_type, EntryStatus::Archived);

        // Expected -> Pending: should move from expected to pending balances
        let expected_to_pending = (
            pending_deltas.pending_credits - expected_deltas.pending_credits,
            pending_deltas.pending_debits - expected_deltas.pending_debits,
            pending_deltas.posted_credits - expected_deltas.posted_credits,
            pending_deltas.posted_debits - expected_deltas.posted_debits,
        );
        assert_eq!(expected_to_pending, (amount, 0, 0, 0));

        // Pending -> Posted: should move from pending to posted balances
        let pending_to_posted = (
            posted_deltas.pending_credits - pending_deltas.pending_credits,
            posted_deltas.pending_debits - pending_deltas.pending_debits,
            posted_deltas.posted_credits - pending_deltas.posted_credits,
            posted_deltas.posted_debits - pending_deltas.posted_debits,
        );
        assert_eq!(pending_to_posted, (-amount, 0, amount, 0));

        // Posted -> Archived: should remove from posted balances only
        let posted_to_archived = (
            archived_deltas.pending_credits - posted_deltas.pending_credits,
            archived_deltas.pending_debits - posted_deltas.pending_debits,
            archived_deltas.posted_credits - posted_deltas.posted_credits,
            archived_deltas.posted_debits - posted_deltas.posted_debits,
        );
        assert_eq!(posted_to_archived, (0, 0, -amount, 0));
    }

    #[test]
    fn test_batch_balance_aggregation_logic() {
        // Test the logic used in batch operations for aggregating balance changes
        use std::collections::HashMap;

        type BalanceChanges = (i64, i64, i64, i64, i64, i64, i64, i64);
        let mut account_changes: HashMap<String, BalanceChanges> = HashMap::new();
        let account_id = "test_account".to_string();

        // Simulate multiple entries affecting the same account
        let entries = vec![
            (100, BalanceDirection::Credit, EntryStatus::Pending),
            (200, BalanceDirection::Debit, EntryStatus::Pending),
            (150, BalanceDirection::Credit, EntryStatus::Posted),
        ];

        for (amount, entry_type, status) in entries {
            let deltas = calculate_balance_deltas(amount, entry_type, status);

            let totals = account_changes
                .entry(account_id.clone())
                .or_insert((0, 0, 0, 0, 0, 0, 0, 0));
            totals.0 += deltas.pending_credits;
            totals.1 += deltas.pending_debits;
            totals.2 += deltas.posted_credits;
            totals.3 += deltas.posted_debits;
            totals.4 += deltas.expected_credits;
            totals.5 += deltas.expected_debits;
            totals.6 += deltas.mismatched_credits;
            totals.7 += deltas.mismatched_debits;
        }

        let final_totals = account_changes.get(&account_id).unwrap();
        // Expected: pending_credits = 100, pending_debits = 200, posted_credits = 150, posted_debits = 0, expected_credits = 0, expected_debits = 0, mismatched_credits = 0, mismatched_debits = 0
        assert_eq!(*final_totals, (100, 200, 150, 0, 0, 0, 0, 0));
    }
}
