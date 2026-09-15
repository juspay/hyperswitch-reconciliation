use async_trait::async_trait;
use common_types::{
    entry::{EntryListFilters, EntryListSortField, ListEntriesFilters},
    pagination::PaginationQuery,
};
use diesel_models::{
    entry::{Entry, EntryNew, EntryPage},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait EntryInterface {
    async fn insert_entry(&self, entry: EntryNew) -> StorageResult<Entry>;
    async fn find_entry_by_id(&self, entry_id: &str) -> StorageResult<Entry>;
    async fn list_entries_by_account_id(
        &self,
        account_id: &str,
        filters: ListEntriesFilters,
    ) -> StorageResult<Vec<Entry>>;
    async fn list_entries_by_transaction_id_and_version(
        &self,
        transaction_id: &str,
        version: Option<i32>,
    ) -> StorageResult<Vec<Entry>>;

    async fn list_expected_entries_by_transaction_id_version_and_account(
        &self,
        transaction_id: &str,
        version: i32,
        account_id: &str,
    ) -> StorageResult<Vec<Entry>>;

    async fn find_non_expected_entry_by_transaction_id_version_and_account(
        &self,
        transaction_id: &str,
        version: i32,
        account_id: &str,
    ) -> StorageResult<Option<Entry>>;

    async fn find_first_pending_source_by_transaction_id_and_version(
        &self,
        transaction_id: &str,
        version: i32,
        source_account_id: &str,
    ) -> StorageResult<Entry>;

    async fn list_entries_by_ids(&self, entry_ids: Vec<String>) -> StorageResult<Vec<Entry>>;

    async fn list_entries(
        &self,
        filters: EntryListFilters,
        pagination: PaginationQuery<EntryListSortField>,
    ) -> StorageResult<EntryPage>;
}

#[async_trait]
impl EntryInterface for Store {
    async fn insert_entry(&self, entry: EntryNew) -> StorageResult<Entry> {
        let conn = self.get_connection().await?;
        entry.insert(&conn).await
    }

    async fn find_entry_by_id(&self, entry_id: &str) -> StorageResult<Entry> {
        let conn = self.get_connection().await?;
        Entry::find_by_entry_id(&conn, entry_id.to_string()).await
    }

    async fn list_entries_by_account_id(
        &self,
        account_id: &str,
        filters: ListEntriesFilters,
    ) -> StorageResult<Vec<Entry>> {
        let conn = self.get_connection().await?;
        Entry::list_by_account_id(&conn, account_id, filters).await
    }

    async fn list_entries_by_transaction_id_and_version(
        &self,
        transaction_id: &str,
        version: Option<i32>,
    ) -> StorageResult<Vec<Entry>> {
        let conn = self.get_connection().await?;
        Entry::list_by_transaction_id_and_version(&conn, transaction_id, version).await
    }

    async fn list_expected_entries_by_transaction_id_version_and_account(
        &self,
        transaction_id: &str,
        version: i32,
        account_id: &str,
    ) -> StorageResult<Vec<Entry>> {
        let conn = self.get_connection().await?;
        Entry::list_expected_by_transaction_id_version_and_account(
            &conn,
            transaction_id,
            version,
            account_id,
        )
        .await
    }

    async fn find_non_expected_entry_by_transaction_id_version_and_account(
        &self,
        transaction_id: &str,
        version: i32,
        account_id: &str,
    ) -> StorageResult<Option<Entry>> {
        let conn = self.get_connection().await?;
        Entry::find_non_expected_by_transaction_id_version_and_account(
            &conn,
            transaction_id,
            version,
            account_id,
        )
        .await
    }

    async fn find_first_pending_source_by_transaction_id_and_version(
        &self,
        transaction_id: &str,
        version: i32,
        source_account_id: &str,
    ) -> StorageResult<Entry> {
        let conn = self.get_connection().await?;
        Entry::find_first_pending_source_by_transaction_id_and_version(
            &conn,
            transaction_id,
            version,
            source_account_id,
        )
        .await
    }

    async fn list_entries_by_ids(&self, entry_ids: Vec<String>) -> StorageResult<Vec<Entry>> {
        let conn = self.get_connection().await?;
        Entry::list_entries_by_ids(&conn, entry_ids).await
    }

    async fn list_entries(
        &self,
        filters: EntryListFilters,
        pagination: PaginationQuery<EntryListSortField>,
    ) -> StorageResult<EntryPage> {
        let conn = self.get_connection().await?;
        Entry::list_paginated(&conn, filters, pagination).await
    }
}
