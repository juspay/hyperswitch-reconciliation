//! Staging entry-related types used across the application

pub mod data;
pub mod status;

pub use data::*;

use crate::date_time::TimeRange;
use common_enums::{BalanceDirection, Currency, DieselStagingEntryStatus};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

#[derive(Debug, Clone)]
pub struct ListStagingEntriesFilters {
    pub status: Option<Vec<DieselStagingEntryStatus>>,
    pub entry_type: Option<Vec<BalanceDirection>>,
    pub account_id: Option<String>,
    pub transformation_history_id: Option<String>,
    pub currency: Option<Vec<Currency>>,
    pub time_range: Option<TimeRange>,
    pub staging_entry_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StagingEntryListFiltersV2 {
    pub status: Option<Vec<DieselStagingEntryStatus>>,
    pub entry_type: Option<Vec<BalanceDirection>>,
    pub account_ids: Option<Vec<String>>,
    pub transformation_config_ids: Option<Vec<String>>,
    pub transformation_history_id: Option<String>,
    pub transformation_history_ids: Option<Vec<String>>,
    pub currency: Option<Vec<Currency>>,
    pub time_range: Option<TimeRange>,
    pub staging_entry_id: Option<String>,
    pub order_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(tag = "sort_field", rename_all = "snake_case")]
pub enum StagingEntryListSortField {
    Id {
        id: Option<String>,
    },
    EffectiveAt {
        cursor_value: Option<StagingEntryEffectiveAtSortField>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct StagingEntryEffectiveAtSortField {
    #[schema(value_type = String)]
    #[serde(with = "common_utils::custom_serde::iso8601_micros")]
    pub effective_at: PrimitiveDateTime,
    pub id: String,
}
