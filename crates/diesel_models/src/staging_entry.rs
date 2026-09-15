use crate::{account::Account, schema::staging_entry, transformation_config::TransformationConfig};
use common_enums::{
    BalanceDirection, Currency, DieselStagingEntryStatus, StagingEntryProcessingMode,
};
use common_types::{
    pagination::CursorSort,
    staging_entry::{
        EditedData, EditedDataV1, PendingData, PendingDataV1, StagingEntryData, StagingEntryDataV2,
        StagingEntryEffectiveAtSortField, StagingEntryListSortField,
    },
    units::MinorUnit,
};
use common_utils::{
    date_time, encryption::Encryption, id_generation::generate_id_with_default_len,
    id_type::MerchantId,
};
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = staging_entry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct StagingEntry {
    pub staging_entry_id: String,
    pub account_id: String,
    pub balance_direction: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub status: DieselStagingEntryStatus,
    pub effective_at: PrimitiveDateTime,
    pub metadata: Encryption,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub processing_mode: StagingEntryProcessingMode,
    pub profile_id: String,
    pub transformation_config_id: String,
    pub transformation_history_id: Option<String>,
    pub merchant_id: MerchantId,
    pub data: StagingEntryData,
    pub id: String,
    pub version: i32,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<DieselStagingEntryStatus>,
    pub discarded_data: Option<StagingEntryData>,
    pub entry_identifier: String,
    pub modified_by: Option<String>,
    pub exposed_metadata: serde_json::Value,
    pub searchable_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = staging_entry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct StagingEntryNew {
    pub staging_entry_id: String,
    pub account_id: String,
    pub balance_direction: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub status: DieselStagingEntryStatus,
    pub effective_at: PrimitiveDateTime,
    pub metadata: Encryption,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub processing_mode: StagingEntryProcessingMode,
    pub profile_id: String,
    pub transformation_config_id: String,
    pub transformation_history_id: Option<String>,
    pub merchant_id: MerchantId,
    pub data: StagingEntryData,
    pub id: String,
    pub version: i32,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<DieselStagingEntryStatus>,
    pub discarded_data: Option<StagingEntryData>,
    pub entry_identifier: String,
    pub modified_by: Option<String>,
    pub exposed_metadata: serde_json::Value,
    pub searchable_metadata: serde_json::Value,
}

impl StagingEntryNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account_id: String,
        merchant_id: MerchantId,
        balance_direction: BalanceDirection,
        amount: MinorUnit,
        currency: Currency,
        effective_at: PrimitiveDateTime,
        metadata: Encryption,
        processing_mode: StagingEntryProcessingMode,
        profile_id: String,
        transformation_config_id: String,
        transformation_history_id: Option<String>,
        entry_identifier: String,
        modified_by: Option<String>,
        exposed_metadata: serde_json::Value,
        searchable_metadata: serde_json::Value,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("stag_ent"); // Primary key (unique per version)
        let staging_entry_id = generate_id_with_default_len("stag_ent_main");
        let data = StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::New,
        )));
        Self {
            id,
            staging_entry_id,
            account_id,
            balance_direction,
            amount,
            currency,
            status: (&data).into(),
            effective_at,
            metadata,
            created_at: now,
            last_modified_at: now,
            processing_mode,
            profile_id,
            transformation_config_id,
            transformation_history_id,
            merchant_id,
            data,
            version: 1,
            discarded_at: None,
            discarded_status: None,
            discarded_data: None,
            entry_identifier,
            modified_by,
            exposed_metadata,
            searchable_metadata,
        }
    }

    pub fn new_versioned(
        previous_staging_entry: StagingEntry,
        update: StagingEntryVersionUpdate,
        modified_by: Option<String>,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("stag_ent"); // Primary key

        let data = match update {
            StagingEntryVersionUpdate::ProcessedUpdate { data }
            | StagingEntryVersionUpdate::NeedsManualReview { data }
            | StagingEntryVersionUpdate::VoidUpdate { data } => data,
        };

        Self {
            id,
            staging_entry_id: previous_staging_entry.staging_entry_id.clone(),
            account_id: previous_staging_entry.account_id.clone(),
            merchant_id: previous_staging_entry.merchant_id.clone(),
            balance_direction: previous_staging_entry.balance_direction,
            amount: previous_staging_entry.amount,
            currency: previous_staging_entry.currency,
            status: (&data).into(),
            effective_at: previous_staging_entry.effective_at,
            metadata: previous_staging_entry.metadata.clone(),
            created_at: now,
            last_modified_at: now,
            processing_mode: previous_staging_entry.processing_mode,
            profile_id: previous_staging_entry.profile_id.clone(),
            transformation_config_id: previous_staging_entry.transformation_config_id.clone(),
            transformation_history_id: previous_staging_entry.transformation_history_id.clone(),
            data,
            version: previous_staging_entry.version + 1,
            discarded_at: None,
            discarded_status: None,
            discarded_data: None,
            entry_identifier: previous_staging_entry.entry_identifier.clone(),
            modified_by,
            exposed_metadata: previous_staging_entry.exposed_metadata.clone(),
            searchable_metadata: previous_staging_entry.searchable_metadata.clone(),
        }
    }

    /// Create new version with updated field values
    pub fn new_updated(
        previous_staging_entry: StagingEntry,
        update: StagingEntryFieldsUpdate,
        modified_by: Option<String>,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("stag_ent");

        // Create EditedData with reason
        let edited_data = StagingEntryData::V2(StagingEntryDataV2::Pending(PendingData::V1(
            PendingDataV1::Edited(EditedData::V1(EditedDataV1 {
                reason: update.reason,
            })),
        )));

        Self {
            id,
            staging_entry_id: previous_staging_entry.staging_entry_id.clone(),
            account_id: update.account_id,
            merchant_id: previous_staging_entry.merchant_id.clone(),
            balance_direction: update.entry_type,
            amount: update.amount,
            currency: update.currency,
            status: (&edited_data).into(),
            effective_at: update.effective_at,
            metadata: update.metadata,
            created_at: now,
            last_modified_at: now,
            processing_mode: previous_staging_entry.processing_mode,
            profile_id: previous_staging_entry.profile_id.clone(),
            transformation_config_id: update.transformation_id,
            transformation_history_id: previous_staging_entry.transformation_history_id.clone(),
            data: edited_data,
            version: previous_staging_entry.version + 1,
            discarded_at: None,
            discarded_status: None,
            discarded_data: None,
            entry_identifier: update.order_id,
            modified_by,
            exposed_metadata: update.exposed_metadata,
            searchable_metadata: update.searchable_metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StagingEntryVersionUpdate {
    ProcessedUpdate { data: StagingEntryData },
    NeedsManualReview { data: StagingEntryData },
    VoidUpdate { data: StagingEntryData },
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = staging_entry)]
pub struct StagingEntryUpdateInternal {
    pub status: Option<DieselStagingEntryStatus>,
    pub discarded_at: Option<Option<PrimitiveDateTime>>,
    pub discarded_status: Option<Option<DieselStagingEntryStatus>>,
    pub data: Option<StagingEntryData>,
    pub discarded_data: Option<Option<StagingEntryData>>,
    pub last_modified_at: PrimitiveDateTime,
}

/// Enum representing different types of staging entry updates
#[derive(Debug, Clone)]
pub enum StagingEntryUpdate {
    /// Discard the transaction with specified data (status will be derived from data)
    DiscardUpdate { original_data: StagingEntryData },
}

/// Field update for staging entries - separate from status-only updates
#[derive(Debug, Clone)]
pub struct StagingEntryFieldsUpdate {
    pub reason: String,
    pub account_id: String,
    pub entry_type: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub effective_at: PrimitiveDateTime,
    pub metadata: Encryption,
    pub transformation_id: String,
    pub order_id: String,
    pub exposed_metadata: serde_json::Value,
    pub searchable_metadata: serde_json::Value,
}

impl From<StagingEntryUpdate> for StagingEntryUpdateInternal {
    fn from(staging_entry_update: StagingEntryUpdate) -> Self {
        let last_modified_at = date_time::now();

        match staging_entry_update {
            StagingEntryUpdate::DiscardUpdate { original_data } => Self {
                status: Some(DieselStagingEntryStatus::Archived),
                discarded_at: Some(Some(last_modified_at)),
                discarded_status: Some(Some((&original_data).into())),
                discarded_data: Some(Some(original_data)),
                data: Some(StagingEntryData::V2(StagingEntryDataV2::Archived)),
                last_modified_at,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct StagingEntryOverviewRow {
    pub account_id: String,
    pub status: DieselStagingEntryStatus,
    pub currency: Currency,
    pub count: i64,
    pub credit_sum: MinorUnit,
    pub debit_sum: MinorUnit,
}

/// Combined view of Staging Entry and Account
#[derive(Debug, Clone)]
pub struct StagingEntryAccountView {
    pub staging_entry: StagingEntry,
    pub account: Account,
    pub transformation_config: TransformationConfig,
}

#[derive(Debug, Clone)]
pub struct StagingEntryAccountViewPage {
    pub views: Vec<StagingEntryAccountView>,
    pub has_more: bool,
}

impl CursorSort<StagingEntryAccountView> for StagingEntryListSortField {
    fn cursor_at(&self, item: &StagingEntryAccountView) -> Self {
        match self {
            Self::Id { .. } => Self::Id {
                id: Some(item.staging_entry.id.clone()),
            },
            Self::EffectiveAt { .. } => Self::EffectiveAt {
                cursor_value: Some(StagingEntryEffectiveAtSortField {
                    effective_at: item.staging_entry.effective_at,
                    id: item.staging_entry.id.clone(),
                }),
            },
        }
    }

    fn cursor_supplied(&self) -> bool {
        match self {
            Self::Id { id } => id.is_some(),
            Self::EffectiveAt { cursor_value } => cursor_value.is_some(),
        }
    }
}
