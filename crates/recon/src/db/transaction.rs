use std::collections::HashMap;

use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use common_enums::{ExpectationStatus, Granularity, SearchDuplicateStatus};
use common_types::{
    date_time::{DefinedTimeRange, TimeRange},
    pagination::PaginationQuery,
    transaction::{TransactionAccountFilters, TransactionListFiltersV2, TransactionListSortField},
};
use common_utils::logging;
use diesel_models::{
    entry::{Entry, EntryNew, EntryUpdate},
    errors::DatabaseError,
    search_duplicate::SearchDuplicateUpdate,
    search_entry_key::SearchEntryKeyNew,
    search_expectation::{SearchExpectation, SearchExpectationNew, SearchExpectationUpdate},
    search_group::{SearchGroup, SearchGroupUpdate},
    transaction::{
        RuleAccountTransactionOverviewRow, Transaction, TransactionAccountView, TransactionNew,
        TransactionOverviewRow, TransactionRuleViewPage, TransactionTimeBucketRow,
        TransactionUpdate,
    },
    PgPooledConn, StorageResult,
};
use error_stack::ResultExt;
use futures::future;
use time::PrimitiveDateTime;

use crate::utils::search_duplicate::update_search_duplicates_for_entries;

use super::Store;

#[async_trait]
pub trait TransactionInterface {
    async fn find_by_id_and_profile(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<Transaction>;

    async fn find_transaction_by_transaction_id_and_version(
        &self,
        transaction_id: &str,
        version: i32,
    ) -> StorageResult<Transaction>;

    async fn find_latest_transaction_by_logical_id_and_profile_id(
        &self,
        transaction_id: &str,
        profile_id: &str,
    ) -> StorageResult<Option<Transaction>>;

    async fn list_by_transaction_id(
        &self,
        transaction_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Transaction>>;

    async fn list_transactions_by_primary_ids_and_profile(
        &self,
        transaction_ids: &[String],
        profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Transaction>>;

    async fn list_with_accounts_and_entries(
        &self,
        profile_id: &str,
        filters: TransactionAccountFilters,
        limit: Option<i64>,
    ) -> StorageResult<Vec<TransactionAccountView>>;

    async fn aggregate_transaction_status_counts_by_rule(
        &self,
        profile_id: &str,
        time_range: TimeRange,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<TransactionOverviewRow>>;

    async fn aggregate_transaction_status_counts_by_rule_and_account(
        &self,
        profile_id: &str,
        time_range: TimeRange,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<RuleAccountTransactionOverviewRow>>;

    async fn list_transactions_v2(
        &self,
        profile_id: &str,
        filter: TransactionListFiltersV2,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        pagination: PaginationQuery<TransactionListSortField>,
    ) -> StorageResult<TransactionRuleViewPage>;

    async fn aggregate_transaction_time_series_by_rule(
        &self,
        profile_id: &str,
        time_range: DefinedTimeRange,
        granularity: Granularity,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<TransactionTimeBucketRow>>;

    async fn insert_transaction(&self, transaction: TransactionNew) -> StorageResult<Transaction>;

    async fn create_transaction_with_entries_and_expectations(
        &self,
        transaction: TransactionNew,
        entries: Vec<EntryNew>,
        search_expectations: Vec<SearchExpectationNew>,
        search_entry_keys: Vec<SearchEntryKeyNew>,
    ) -> StorageResult<(Transaction, Vec<Entry>, Vec<SearchExpectation>)>;

    async fn discard_transaction_with_entries_and_expectations(
        &self,
        conn: &PgPooledConn,
        transaction: Transaction,
    ) -> StorageResult<(
        Transaction,
        Vec<Entry>,
        Vec<SearchExpectation>,
        Option<SearchGroup>,
    )>;
}

#[async_trait]
impl TransactionInterface for Store {
    async fn find_by_id_and_profile(
        &self,
        id: &str,
        profile_id: &str,
    ) -> StorageResult<Transaction> {
        let conn = self.get_connection().await?;
        Transaction::find_by_id_and_profile(&conn, id.to_string(), profile_id.to_string()).await
    }

    async fn find_transaction_by_transaction_id_and_version(
        &self,
        transaction_id: &str,
        version: i32,
    ) -> StorageResult<Transaction> {
        let conn = self.get_connection().await?;
        Transaction::find_by_transaction_id_and_version(&conn, transaction_id, version).await
    }

    async fn find_latest_transaction_by_logical_id_and_profile_id(
        &self,
        transaction_id: &str,
        profile_id: &str,
    ) -> StorageResult<Option<Transaction>> {
        let conn = self.get_connection().await?;
        Transaction::find_latest_version_by_logical_id_and_profile_id(
            &conn,
            transaction_id,
            profile_id,
        )
        .await
    }

    async fn list_by_transaction_id(
        &self,
        transaction_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Transaction>> {
        let conn = self.get_connection().await?;
        Transaction::list_by_transaction_id(&conn, transaction_id, limit).await
    }

    async fn list_transactions_by_primary_ids_and_profile(
        &self,
        primary_transaction_ids: &[String],
        profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<Transaction>> {
        let conn = self.get_connection().await?;
        Transaction::list_transactions_by_primary_ids_and_profile(
            &conn,
            primary_transaction_ids,
            profile_id,
            limit,
        )
        .await
    }

    async fn list_with_accounts_and_entries(
        &self,
        profile_id: &str,
        filters: TransactionAccountFilters,
        limit: Option<i64>,
    ) -> StorageResult<Vec<TransactionAccountView>> {
        let conn = self.get_connection().await?;
        Transaction::list_with_accounts_and_entries(&conn, profile_id, filters, limit).await
    }

    async fn aggregate_transaction_status_counts_by_rule(
        &self,
        profile_id: &str,
        time_range: TimeRange,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<TransactionOverviewRow>> {
        let conn = self.get_connection().await?;
        Transaction::aggregate_status_counts_by_rule(
            &conn,
            profile_id,
            time_range,
            rule_missing_threshold_map,
            rule_ids,
        )
        .await
    }

    async fn aggregate_transaction_status_counts_by_rule_and_account(
        &self,
        profile_id: &str,
        time_range: TimeRange,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<RuleAccountTransactionOverviewRow>> {
        let conn = self.get_connection().await?;
        Transaction::aggregate_status_counts_by_rule_and_account(
            &conn,
            profile_id,
            time_range,
            rule_missing_threshold_map,
            rule_ids,
        )
        .await
    }

    async fn list_transactions_v2(
        &self,
        profile_id: &str,
        filter: TransactionListFiltersV2,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        pagination: PaginationQuery<TransactionListSortField>,
    ) -> StorageResult<TransactionRuleViewPage> {
        let conn = self.get_connection().await?;
        Transaction::list_v2_paginated(
            &conn,
            profile_id,
            filter,
            rule_missing_threshold_map,
            pagination,
        )
        .await
    }

    async fn aggregate_transaction_time_series_by_rule(
        &self,
        profile_id: &str,
        time_range: DefinedTimeRange,
        granularity: Granularity,
        rule_missing_threshold_map: HashMap<String, Option<PrimitiveDateTime>>,
        rule_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<TransactionTimeBucketRow>> {
        let conn = self.get_connection().await?;
        Transaction::aggregate_time_series_status_counts_by_rule(
            &conn,
            profile_id,
            time_range,
            granularity,
            rule_missing_threshold_map,
            rule_ids,
        )
        .await
    }

    async fn insert_transaction(&self, transaction: TransactionNew) -> StorageResult<Transaction> {
        let conn = self.get_connection().await?;
        transaction.insert(&conn).await
    }

    async fn create_transaction_with_entries_and_expectations(
        &self,
        transaction: TransactionNew,
        entries: Vec<EntryNew>,
        search_expectations: Vec<SearchExpectationNew>,
        search_entry_keys: Vec<SearchEntryKeyNew>,
    ) -> StorageResult<(Transaction, Vec<Entry>, Vec<SearchExpectation>)> {
        let conn = self.get_connection().await?;

        conn.transaction_async(|connection_pool| async move {
            // Insert all entries first
            let mut created_entries = Vec::new();
            for entry in entries {
                let created_entry = entry
                    .insert(&connection_pool)
                    .await
                    .map_err(|_| DatabaseError::Others)?;
                created_entries.push(created_entry);
            }

            SearchEntryKeyNew::batch_insert(search_entry_keys, &connection_pool)
                .await
                .map_err(|_| DatabaseError::Others)?;

            // Insert all search expectations
            let mut created_expectations = Vec::new();
            for expectation in search_expectations {
                let created_expectation = expectation
                    .insert(&connection_pool)
                    .await
                    .map_err(|_| DatabaseError::Others)?;
                created_expectations.push(created_expectation);
            }

            // Finally, insert the transaction
            let created_transaction = transaction
                .insert(&connection_pool)
                .await
                .map_err(|_| DatabaseError::Others)?;

            Ok::<_, DatabaseError>((created_transaction, created_entries, created_expectations))
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable("Failed to create transaction with entries and expectations atomically")
    }

    async fn discard_transaction_with_entries_and_expectations(
        &self,
        conn: &PgPooledConn,
        transaction: Transaction,
    ) -> StorageResult<(
        Transaction,
        Vec<Entry>,
        Vec<SearchExpectation>,
        Option<SearchGroup>,
    )> {
        conn.transaction_async(|connection_pool| async move {
            // First, archive the existing transaction
            let archive_update = TransactionUpdate::DiscardUpdate {
                original_data: transaction.data.clone(),
            };
            Transaction::update_by_id(&connection_pool, &transaction.id, archive_update.into())
                .await
                .map_err(|e| {
                    logging::error!("Failed to archive old transaction: {:?}", e);
                    *e.current_context()
                })?;

            // Get entries for the existing transaction version
            let old_entries = Entry::list_by_transaction_id_and_version(
                &connection_pool,
                &transaction.transaction_id,
                Some(transaction.version),
            )
            .await
            .map_err(|e| {
                logging::error!("Failed to fetch entries for transaction: {:?}", e);
                *e.current_context()
            })?;

            future::try_join_all(old_entries.iter().map(|entry| {
                let conn = &connection_pool;
                let transaction_id = transaction.transaction_id.clone();
                async move {
                    Entry::discard_entry_by_id(
                        conn,
                        &entry.id.clone(),
                        EntryUpdate::DiscardUpdate {
                            original_data: entry.data.clone(),
                        }
                        .into(),
                    )
                    .await
                    .map_err(|e| {
                        logging::error!(
                            "Failed to archive entry {} for transaction {}: {:?}",
                            entry.id.clone(),
                            transaction_id,
                            e
                        );
                        *e.current_context()
                    })?;
                    Ok::<_, DatabaseError>(())
                }
            }))
            .await?;

            update_search_duplicates_for_entries(
                &connection_pool,
                &old_entries,
                &SearchDuplicateUpdate::StatusUpdate {
                    status: SearchDuplicateStatus::Archived,
                },
            )
            .await?;

            let search_group = SearchGroup::find_by_primary_transaction_id_and_profile_id(
                &connection_pool,
                &transaction.id,
                &transaction.profile_id,
            )
            .await
            .map_err(|e| {
                logging::error!(
                    "Failed to fetch search group for transaction {}: {:?}",
                    transaction.id,
                    e
                );
                *e.current_context()
            })?;

            if let Some(ref search_group) = search_group {
                SearchGroup::discard_by_id(
                    &connection_pool,
                    &search_group.id,
                    SearchGroupUpdate::DiscardUpdate.into(),
                )
                .await
                .map_err(|e| {
                    logging::error!(
                        "Failed to archive search group {} for transaction {}: {:?}",
                        search_group.id,
                        transaction.id,
                        e
                    );
                    *e.current_context()
                })?;
            }

            // Archive search expectations for all entries
            let all_entry_ids: Vec<String> =
                old_entries.iter().map(|entry| entry.id.clone()).collect();

            let mut search_expectations = Vec::new();

            if !all_entry_ids.is_empty() {
                search_expectations =
                    SearchExpectation::batch_update_status_by_primary_transaction_ids(
                        &connection_pool,
                        vec![transaction.id.clone()],
                        SearchExpectationUpdate::StatusUpdate {
                            status: ExpectationStatus::Archived,
                        }
                        .into(),
                    )
                    .await
                    .map_err(|e| {
                        logging::error!(
                            "Failed to archive search expectations for transaction {}: {:?}",
                            transaction.transaction_id,
                            e
                        );
                        *e.current_context()
                    })?;
            }
            Ok::<_, DatabaseError>((transaction, old_entries, search_expectations, search_group))
        })
        .await
        .change_context(DatabaseError::Others)
        .attach_printable("Failed to discard transaction with entries and expectations atomically")
    }
}
