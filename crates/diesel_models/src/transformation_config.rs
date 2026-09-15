use crate::schema::transformation_config;
use common_types::transformation::TransformationConfigData;
use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = transformation_config)]
pub struct TransformationConfig {
    pub id: String,
    pub profile_id: String,
    pub ingestion_config_id: String,
    pub account_id: String,
    pub name: String,
    pub config: TransformationConfigData,
    pub is_active: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub metadata_schema_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = transformation_config)]
pub struct TransformationConfigNew {
    pub id: String,
    pub profile_id: String,
    pub ingestion_config_id: String,
    pub account_id: String,
    pub name: String,
    pub config: TransformationConfigData,
    pub is_active: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub metadata_schema_id: String,
}

impl TransformationConfigNew {
    pub fn new(
        profile_id: String,
        ingestion_config_id: String,
        account_id: String,
        name: String,
        config: TransformationConfigData,
        is_active: bool,
        metadata_schema_id: String,
    ) -> Self {
        let now = date_time::now();
        let id = generate_id_with_default_len("tran_conf");

        Self {
            id,
            profile_id,
            ingestion_config_id,
            account_id,
            name,
            config,
            is_active,
            created_at: now,
            last_modified_at: now,
            metadata_schema_id,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = transformation_config)]
pub struct TransformationConfigUpdateInternal {
    pub name: Option<String>,
    pub config: Option<TransformationConfigData>,
    pub is_active: Option<bool>,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Clone)]
pub enum TransformationConfigUpdate {
    NameUpdate { name: String },
    ConfigUpdate { config: TransformationConfigData },
    ActiveStatusUpdate { is_active: bool },
}

impl From<TransformationConfigUpdate> for TransformationConfigUpdateInternal {
    fn from(transformation_config_update: TransformationConfigUpdate) -> Self {
        let last_modified_at = date_time::now();

        match transformation_config_update {
            TransformationConfigUpdate::NameUpdate { name } => Self {
                name: Some(name),
                config: None,
                is_active: None,
                last_modified_at,
            },
            TransformationConfigUpdate::ConfigUpdate { config } => Self {
                name: None,
                config: Some(config),
                is_active: None,
                last_modified_at,
            },
            TransformationConfigUpdate::ActiveStatusUpdate { is_active } => Self {
                name: None,
                config: None,
                is_active: Some(is_active),
                last_modified_at,
            },
        }
    }
}
