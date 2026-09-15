use common_enums::DieselStagingEntryStatus;
use common_types::{
    date_time::TimeRange,
    pagination::PaginationQuery,
    staging_entry::{
        ListStagingEntriesFilters, StagingEntryListFiltersV2, StagingEntryListSortField,
    },
};
use diesel_models::{
    staging_entry::{
        StagingEntry, StagingEntryAccountView, StagingEntryAccountViewPage, StagingEntryNew,
        StagingEntryOverviewRow, StagingEntryUpdate,
    },
    StorageResult,
};

use super::Store;

impl Store {
    pub async fn insert_staging_entry(
        &self,
        staging_entry: StagingEntryNew,
    ) -> StorageResult<StagingEntry> {
        let conn = self.get_connection().await?;
        staging_entry.insert(&conn).await
    }

    pub async fn batch_insert_staging_entries(
        &self,
        staging_entries: Vec<StagingEntryNew>,
    ) -> StorageResult<Vec<StagingEntry>> {
        let conn = self.get_connection().await?;
        StagingEntryNew::batch_insert_chunked(&conn, staging_entries).await
    }

    pub async fn find_staging_entry_by_id(&self, id: String) -> StorageResult<StagingEntry> {
        let conn = self.get_connection().await?;
        StagingEntry::find_by_id(&conn, id).await
    }

    pub async fn find_staging_entry_by_id_and_profile(
        &self,
        id: String,
        profile_id: String,
    ) -> StorageResult<StagingEntry> {
        let conn = self.get_connection().await?;
        StagingEntry::find_by_id_and_profile(&conn, id, profile_id).await
    }

    pub async fn list_staging_entries_by_primary_ids_and_profile(
        &self,
        ids: &[String],
        profile_id: &str,
        limit: Option<i64>,
    ) -> StorageResult<Vec<StagingEntry>> {
        let conn = self.get_connection().await?;
        StagingEntry::list_staging_entries_by_primary_ids_and_profile(&conn, ids, profile_id, limit)
            .await
    }

    pub async fn update_by_id(
        &self,
        id: &str,
        staging_entry_update: StagingEntryUpdate,
    ) -> StorageResult<StagingEntry> {
        let conn = self.get_connection().await?;
        let staging_entry_update_internal = staging_entry_update.into();
        StagingEntry::update_by_id(&conn, id, staging_entry_update_internal).await
    }

    pub async fn list_all_staging_entries(
        &self,
        status: Option<DieselStagingEntryStatus>,
        limit: u16,
    ) -> StorageResult<Vec<StagingEntry>> {
        let conn = self.get_connection().await?;
        StagingEntry::list_all_staging_entries(&conn, status, limit).await
    }

    pub async fn list_staging_entries_by_profile_id(
        &self,
        profile_id: &str,
        filters: ListStagingEntriesFilters,
        limit: Option<i64>,
    ) -> StorageResult<Vec<StagingEntryAccountView>> {
        let conn = self.get_connection().await?;
        StagingEntry::list_with_account_by_profile_id(&conn, profile_id, filters, limit).await
    }

    pub async fn list_staging_entries_v2(
        &self,
        profile_id: &str,
        filters: StagingEntryListFiltersV2,
        pagination: PaginationQuery<StagingEntryListSortField>,
    ) -> StorageResult<StagingEntryAccountViewPage> {
        let conn = self.get_connection().await?;
        StagingEntry::list_v2_paginated(&conn, profile_id, filters, pagination).await
    }

    pub async fn count_pending_staging_entries_by_profile(
        &self,
        profile_id: &str,
    ) -> StorageResult<i64> {
        let conn = self.get_connection().await?;
        StagingEntry::count_pending_by_profile(&conn, profile_id).await
    }

    pub async fn aggregate_staging_entry_status_counts_by_account(
        &self,
        profile_id: &str,
        time_range: TimeRange,
        account_ids: Option<Vec<String>>,
        transformation_history_ids: Option<Vec<String>>,
    ) -> StorageResult<Vec<StagingEntryOverviewRow>> {
        let conn = self.get_connection().await?;
        StagingEntry::aggregate_status_counts_by_account(
            &conn,
            profile_id,
            time_range,
            account_ids,
            transformation_history_ids,
        )
        .await
    }
}
