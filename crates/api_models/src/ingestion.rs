use std::path::PathBuf;

use actix_multipart::form::{bytes::Bytes, MultipartForm};
use common_enums::IngestionStatus;
use common_utils::crypto::EncryptableName;
use common_utils::id_type::MerchantId;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;
/// Request to create a new ingestion configuration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestionConfigCreateRequest {
    #[schema(value_type = String, example = "merch_123456")]
    pub merchant_id: MerchantId,

    #[schema(example = "pro_123456")]
    pub profile_id: String,

    #[schema(example = "adyen_webhook")]
    pub name: String,

    #[schema(example = "Adyen webhook ingestion for daily settlements")]
    pub description: Option<String>,

    #[schema(example = "acc_123456")]
    pub account_id: String,

    pub config: IngestionConfigData,

    #[schema(example = true)]
    pub is_active: bool,
}

/// Request to update an ingestion configuration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestionConfigUpdateRequest {
    #[schema(example = true)]
    pub is_active: Option<bool>,

    pub config: Option<IngestionConfigData>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IngestionConfigData {
    Adyen {
        #[schema(value_type = String, example = "hmac_secret")]
        hmac_secret: Secret<String>,
        webhook_basic_auth_username: String,
        #[schema(value_type = String, example = "webhook_basic_auth_password")]
        webhook_basic_auth_password: Secret<String>,
        report_basic_auth_username: String,
        #[schema(value_type = String, example = "report_basic_auth_password")]
        report_basic_auth_password: Secret<String>,
    },
    SftpInternal {
        #[schema(value_type = String, example = "path/to/file")]
        file_path: PathBuf,
    },
    Manual,
}

/// Request for manual file upload to ingestion
#[derive(Debug, MultipartForm, ToSchema)]
pub struct IngestionFileUploadRequest {
    /// File field containing the binary data
    #[schema(value_type = String, format = "binary")]
    #[multipart(limit = "25MB")]
    pub file: Bytes,
}

/// Response for ingestion file upload
#[derive(Debug, Serialize, ToSchema)]
pub struct IngestionFileUploadResponse {
    /// Ingestion history ID created for this upload
    pub ingestion_history_id: String,
    /// File path where the file was stored
    pub file_path: String,
    /// Success message
    pub message: String,
}

/// Internal request structure for multipart file processing
#[derive(Debug)]
pub struct ProcessedIngestionFileRequest {
    pub file_data: Vec<u8>,
    pub file_name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IngestionUploadType {
    Adyen,
    SftpInternal,
    Manual,
}

/// Response for ingestion configuration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestionConfigResponse {
    pub ingestion_id: String,
    pub account_id: String,
    pub is_active: bool,
    pub name: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_synced_at: Option<PrimitiveDateTime>,
    pub data: IngestionConnectionData,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "ingestion_type", rename_all = "snake_case")]
pub enum IngestionConnectionData {
    Adyen {
        #[schema(value_type = String, example = "hmac_secret")]
        hmac_secret: EncryptableName,
        webhook_basic_auth_username: String,
        #[schema(value_type = String, example = "webhook_basic_auth_password")]
        webhook_basic_auth_password: EncryptableName,
        report_basic_auth_username: String,
        #[schema(value_type = String, example = "report_basic_auth_password")]
        report_basic_auth_password: EncryptableName,
    },
    SftpInternal {
        #[schema(value_type = String, example = "path/to/file")]
        file_path: PathBuf,
    },
    Manual,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestionHistoryResponse {
    pub id: String,
    pub ingestion_id: String,
    pub ingestion_history_id: String,
    pub file_name: Option<String>,
    pub account_id: String,
    pub status: IngestionStatus,
    pub upload_type: IngestionUploadType,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub ingestion_name: String,
    pub version: i32,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub discarded_at: Option<PrimitiveDateTime>,
    pub discarded_status: Option<IngestionStatus>,
}

/// List response for ingestion configurations
pub type IngestionConfigListResponse = Vec<IngestionConfigResponse>;

/// List response for ingestion history
pub type IngestionHistoryListResponse = Vec<IngestionHistoryResponse>;
