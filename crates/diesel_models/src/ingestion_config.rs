use crate::schema::ingestion_config;
use common_utils::{id_generation::generate_id_with_default_len, id_type::MerchantId};
use diesel::{
    deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb, AsChangeset, Insertable,
    Queryable,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

pub mod adyen;
pub mod sftp_internal;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = ingestion_config)]
pub struct IngestionConfig {
    pub id: String,
    pub profile_id: String,
    pub account_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub config: IngestionConfigData,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub merchant_id: MerchantId,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = ingestion_config)]
pub struct IngestionConfigNew {
    pub id: String,
    pub profile_id: String,
    pub account_id: String,
    pub name: String,
    pub description: Option<String>,
    pub config: IngestionConfigData,
    pub is_active: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub merchant_id: MerchantId,
}

impl IngestionConfigNew {
    pub fn new(
        profile_id: String,
        account_id: String,
        name: String,
        description: Option<String>,
        config: IngestionConfigData,
        is_active: bool,
        merchant_id: MerchantId,
    ) -> Self {
        let now = common_utils::date_time::now();
        let id = generate_id_with_default_len("ing");

        Self {
            id,
            profile_id,
            account_id,
            name,
            description,
            config,
            is_active,
            created_at: now,
            last_modified_at: now,
            merchant_id,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = ingestion_config)]
pub struct IngestionConfigUpdateInternal {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<IngestionConfigData>,
    pub is_active: Option<bool>,
    pub last_modified_at: PrimitiveDateTime,
}

pub enum IngestionConfigUpdate {
    UpdateStatus { is_active: bool },
    UpdateConfig { config: IngestionConfigData },
}

impl From<IngestionConfigUpdate> for IngestionConfigUpdateInternal {
    fn from(update: IngestionConfigUpdate) -> Self {
        match update {
            IngestionConfigUpdate::UpdateStatus { is_active } => Self {
                name: None,
                description: None,
                config: None,
                is_active: Some(is_active),
                last_modified_at: common_utils::date_time::now(),
            },
            IngestionConfigUpdate::UpdateConfig { config } => Self {
                name: None,
                description: None,
                config: Some(config),
                is_active: None,
                last_modified_at: common_utils::date_time::now(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "ingestion_config_version", rename_all = "snake_case")]
pub enum IngestionConfigData {
    V1(IngestionConfigV1),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "ingestion_type", rename_all = "snake_case")]
pub enum IngestionConfigV1 {
    Psp(IngestionConfigPsp),
    Manual,
    SftpInternal(sftp_internal::SftpInternalIngestionConfig),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "psp", rename_all = "snake_case")]
pub enum IngestionConfigPsp {
    Adyen(adyen::AdyenIngestionConfig),
}

common_utils::impl_to_sql_from_sql_json!(IngestionConfigData);
