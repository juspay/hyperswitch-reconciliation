use crate::{schema::transformation_history, transformation_config::TransformationConfig};
use common_enums::TransformationStatus;
use common_types::transformation::TransformationData;
use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = transformation_history)]
pub struct TransformationHistory {
    pub id: String,
    pub transformation_config_id: String,
    pub ingestion_history_id: String,
    pub profile_id: String,
    pub account_id: String,
    pub status: TransformationStatus,
    pub data: Option<TransformationData>,
    pub processed_at: Option<PrimitiveDateTime>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = transformation_history)]
pub struct TransformationHistoryNew {
    pub id: String,
    pub transformation_config_id: String,
    pub ingestion_history_id: String,
    pub profile_id: String,
    pub account_id: String,
    pub status: TransformationStatus,
    pub data: Option<TransformationData>,
    pub processed_at: Option<PrimitiveDateTime>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

impl TransformationHistoryNew {
    pub fn new(
        transformation_config_id: String,
        ingestion_history_id: String,
        profile_id: String,
        account_id: String,
        status: TransformationStatus,
        data: Option<TransformationData>,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("tran_his");

        Self {
            id,
            transformation_config_id,
            ingestion_history_id,
            profile_id,
            account_id,
            status,
            data,
            processed_at: None,
            created_at: now,
            last_modified_at: now,
        }
    }
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = transformation_history)]
pub struct TransformationHistoryUpdateInternal {
    pub status: TransformationStatus,
    pub data: Option<TransformationData>,
    pub processed_at: Option<PrimitiveDateTime>,
    pub last_modified_at: PrimitiveDateTime,
}

pub enum TransformationHistoryUpdate {
    Processing,
    Processed { processed_data: TransformationData },
    Failed { failure_data: TransformationData },
}

impl From<TransformationHistoryUpdate> for TransformationHistoryUpdateInternal {
    fn from(update: TransformationHistoryUpdate) -> Self {
        let now = date_time::now();
        match update {
            TransformationHistoryUpdate::Processing => Self {
                status: TransformationStatus::Processing,
                data: None,
                processed_at: None,
                last_modified_at: now,
            },
            TransformationHistoryUpdate::Processed { processed_data } => Self {
                status: TransformationStatus::Processed,
                data: Some(processed_data),
                processed_at: Some(now),
                last_modified_at: now,
            },
            TransformationHistoryUpdate::Failed { failure_data } => Self {
                status: TransformationStatus::Failed,
                data: Some(failure_data),
                processed_at: Some(now),
                last_modified_at: now,
            },
        }
    }
}

/// Combined view of Transformation History and Config
#[derive(Debug, Clone)]
pub struct TransformationConfigHistoryView {
    pub config: TransformationConfig,
    pub history: TransformationHistory,
}
