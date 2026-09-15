use crate::{ingestion_config::IngestionConfig, schema::ingestion_history};
use common_enums::IngestionStatus;
use common_types::ingestion::{IngestionData, RawPayload, TriggerData};
use common_utils::id_generation::generate_id_with_default_len;
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = ingestion_history)]
pub struct IngestionHistory {
    pub ingestion_history_id: String,
    pub ingestion_config_id: String,
    pub profile_id: String,
    pub status: IngestionStatus,
    pub data: Option<IngestionData>,
    pub processed_at: Option<PrimitiveDateTime>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub trigger_data: Option<TriggerData>,
    pub raw_payload: RawPayload,
    pub account_id: String,
    pub id: String,
    pub version: i32,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<IngestionStatus>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = ingestion_history)]
pub struct IngestionHistoryNew {
    pub ingestion_history_id: String,
    pub ingestion_config_id: String,
    pub profile_id: String,
    pub status: IngestionStatus,
    pub data: Option<IngestionData>,
    pub processed_at: Option<PrimitiveDateTime>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub trigger_data: Option<TriggerData>,
    pub raw_payload: RawPayload,
    pub account_id: String,
    pub id: String,
    pub version: i32,
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<IngestionStatus>,
}

impl IngestionHistoryNew {
    pub fn new(
        ingestion_id: String,
        profile_id: String,
        account_id: String,
        status: IngestionStatus,
        trigger_data: Option<TriggerData>,
        data: Option<IngestionData>,
        raw_payload: RawPayload,
    ) -> Self {
        let now = common_utils::date_time::now();
        let id = generate_id_with_default_len("ing_hist");
        let ingestion_history_id = generate_id_with_default_len("ing_hist_main");

        Self {
            id,
            ingestion_history_id,
            ingestion_config_id: ingestion_id,
            profile_id,
            account_id,
            status,
            processed_at: None,
            created_at: now,
            last_modified_at: now,
            trigger_data,
            data,
            raw_payload,
            version: 1,
            discarded_at: None,
            discarded_status: None,
        }
    }

    pub fn new_versioned(
        previous: IngestionHistory,
        update: IngestionHistoryVersionUpdate,
    ) -> Self {
        let now = common_utils::date_time::now();
        let id = generate_id_with_default_len("ing_hist");

        let (status, processed_at, trigger_data, data) = match update {
            IngestionHistoryVersionUpdate::ProcessingUpdate { trigger_data } => (
                IngestionStatus::Processing,
                None,
                Some(trigger_data),
                previous.data,
            ),
            IngestionHistoryVersionUpdate::ProcessedUpdate { data } => (
                IngestionStatus::Processed,
                Some(now),
                previous.trigger_data,
                Some(data),
            ),
            IngestionHistoryVersionUpdate::FailedUpdate { data } => (
                IngestionStatus::Failed,
                None,
                previous.trigger_data,
                Some(data),
            ),
        };

        Self {
            id,
            ingestion_history_id: previous.ingestion_history_id,
            ingestion_config_id: previous.ingestion_config_id,
            profile_id: previous.profile_id,
            account_id: previous.account_id,
            status,
            processed_at,
            created_at: now,
            last_modified_at: now,
            trigger_data,
            data,
            raw_payload: previous.raw_payload,
            version: previous.version + 1,
            discarded_at: None,
            discarded_status: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum IngestionHistoryVersionUpdate {
    ProcessingUpdate { trigger_data: TriggerData },
    ProcessedUpdate { data: IngestionData },
    FailedUpdate { data: IngestionData },
}

#[derive(Debug, Clone)]
pub enum IngestionHistoryUpdate {
    DiscardUpdate { original_status: IngestionStatus },
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = ingestion_history)]
pub struct IngestionHistoryUpdateInternal {
    pub status: IngestionStatus,
    pub discarded_at: PrimitiveDateTime,
    pub discarded_status: IngestionStatus,
    pub last_modified_at: PrimitiveDateTime,
}

impl From<IngestionHistoryUpdate> for IngestionHistoryUpdateInternal {
    fn from(update: IngestionHistoryUpdate) -> Self {
        let now = common_utils::date_time::now();

        match update {
            IngestionHistoryUpdate::DiscardUpdate { original_status } => Self {
                status: IngestionStatus::Discarded,
                discarded_at: now,
                discarded_status: original_status,
                last_modified_at: now,
            },
        }
    }
}

/// Combined view of Ingestion History and Config
#[derive(Debug, Clone)]
pub struct IngestionConfigHistoryView {
    pub config: IngestionConfig,
    pub history: IngestionHistory,
}
