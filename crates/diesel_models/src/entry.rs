use crate::schema::entry;
use common_enums::{BalanceDirection, Currency, EntryStatus};
use common_types::{
    entry::{EntryData, EntryDataV2, EntryEffectiveAtSortField, EntryListSortField},
    pagination::CursorSort,
    units::MinorUnit,
};
use common_utils::{
    date_time, encryption::Encryption, id_generation::generate_id_with_default_len,
    id_type::MerchantId,
};
use diesel::{prelude::Insertable, AsChangeset, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, Clone, PartialEq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = entry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Entry {
    pub id: String,
    pub account_id: String,
    pub logical_transaction_id: String,
    pub balance_direction: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub status: EntryStatus,
    pub effective_at: PrimitiveDateTime,
    pub metadata: Encryption,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<EntryStatus>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub transaction_version: i32,
    pub merchant_id: MerchantId,
    pub data: EntryData,
    pub discarded_data: Option<EntryData>,
    pub primary_staging_entry_id: Option<String>,
    pub transformation_config_id: Option<String>,
    pub entry_identifier: String,
    pub exposed_metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct EntryPage {
    pub entries: Vec<Entry>,
    pub has_more: bool,
}

impl CursorSort<Entry> for EntryListSortField {
    fn cursor_at(&self, item: &Entry) -> Self {
        match self {
            Self::Id { .. } => Self::Id {
                id: Some(item.id.clone()),
            },
            Self::EffectiveAt { .. } => Self::EffectiveAt {
                cursor_value: Some(EntryEffectiveAtSortField {
                    effective_at: item.effective_at,
                    id: item.id.clone(),
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

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = entry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EntryNew {
    pub id: String,
    pub account_id: String,
    pub logical_transaction_id: String,
    pub balance_direction: BalanceDirection,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub status: EntryStatus,
    pub effective_at: PrimitiveDateTime,
    pub metadata: Encryption,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<EntryStatus>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub transaction_version: i32,
    pub merchant_id: MerchantId,
    pub data: EntryData,
    pub discarded_data: Option<EntryData>,
    pub primary_staging_entry_id: Option<String>,
    pub transformation_config_id: Option<String>,
    pub entry_identifier: String,
    pub exposed_metadata: serde_json::Value,
}

impl EntryNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account_id: String,
        logical_transaction_id: String,
        balance_direction: BalanceDirection,
        amount: MinorUnit,
        currency: Currency,
        status: EntryStatus,
        effective_at: PrimitiveDateTime,
        merchant_id: MerchantId,
        metadata: Encryption,
        transaction_version: i32,
        data: EntryData,
        primary_staging_entry_id: Option<String>,
        transformation_config_id: Option<String>,
        entry_identifier: String,
        exposed_metadata: serde_json::Value,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("entry");
        Self {
            id,
            account_id,
            logical_transaction_id,
            balance_direction,
            amount,
            currency,
            status,
            effective_at,
            metadata,
            discarded_at: None,
            discarded_status: None,
            created_at: now,
            last_modified_at: now,
            transaction_version,
            merchant_id,
            data,
            discarded_data: None,
            primary_staging_entry_id,
            transformation_config_id,
            entry_identifier,
            exposed_metadata,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = entry)]
pub struct EntryUpdateInternal {
    pub status: Option<EntryStatus>,
    pub data: Option<EntryData>,
    pub discarded_at: Option<Option<PrimitiveDateTime>>,
    pub discarded_status: Option<Option<EntryStatus>>,
    pub discarded_data: Option<Option<EntryData>>,
    pub last_modified_at: PrimitiveDateTime,
}

/// Updates the single residual Expected entry after appending a new source entry.
#[derive(Debug, AsChangeset)]
#[diesel(table_name = entry)]
pub struct ExpectedEntryUpdateInternal {
    pub balance_direction: BalanceDirection,
    pub amount: MinorUnit,
    pub effective_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Clone)]
pub enum EntryUpdate {
    DiscardUpdate { original_data: EntryData },
}

impl From<EntryUpdate> for EntryUpdateInternal {
    fn from(entry_update: EntryUpdate) -> Self {
        let now = date_time::now();
        match entry_update {
            EntryUpdate::DiscardUpdate { original_data } => Self {
                status: Some(EntryStatus::Archived),
                data: Some(EntryData::V2(EntryDataV2::Archived)),
                discarded_at: Some(Some(now)),
                discarded_status: Some(Some((&original_data).into())),
                discarded_data: Some(Some(original_data)),
                last_modified_at: now,
            },
        }
    }
}
