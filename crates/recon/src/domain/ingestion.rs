use actix_web::http::header::HeaderMap;
use common_types::ingestion::{
    manual::ManualTriggerData, RawPayload, RawPayloadV1, TriggerData, TriggerDataV1,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use recon_domain_models::{
    behaviour::ReverseConversion,
    ingestion::sftp_internal::SftpInternalIngestionConfig as DomainSftpInternalIngestionConfig,
    ApplicationEncryptionInterface, EncryptionHandler,
};
use serde::{Deserialize, Serialize};

use diesel_models::ingestion_config::{IngestionConfigData, IngestionConfigPsp, IngestionConfigV1};

pub mod adyen;
pub mod manual;
pub mod sftp_internal;

use crate::{
    domain::ingestion::adyen::parse_adyen_payload,
    errors::ingestion::{
        DownloadStageResult, IngestionError, IngestionResult, ParseStageError, ParseStageResult,
    },
    routes::AppState,
};

/// Unified context that ensures config and payload types match
#[derive(Clone, Debug)]
pub enum IngestionContext {
    Adyen(Box<adyen::AdyenIngestionContext>),
    Manual {
        file_data: Vec<u8>,
        file_name: String,
    },
    SftpInternal(Box<sftp_internal::SftpInternalIngestionContext>),
}

/// Result of downloading a file from an external source
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileDownloadResult {
    pub file_content: Vec<u8>,
    pub file_name: String,
}

/// Context containing all information needed to download a file
/// This is created from IngestionContext after parsing the trigger data
#[derive(Clone, Debug)]
pub enum FileDownloadContext {
    Adyen(adyen::AdyenFileDownloadContext),
    Manual(manual::ManualFileDownloadContext),
    SftpInternal(sftp_internal::SftpInternalFileDownloadContext),
}

impl FileDownloadContext {
    /// Download the file using the context information
    pub async fn download_file(&self, state: &AppState) -> DownloadStageResult<FileDownloadResult> {
        match self {
            Self::Adyen(adyen_context) => {
                adyen::download_adyen_report(adyen_context, &state.config.proxy).await
            }
            Self::Manual(manual_context) => Ok(FileDownloadResult {
                file_content: manual_context.file_data.clone(),
                file_name: manual_context.file_name.clone(),
            }),
            Self::SftpInternal(sftp_context) => {
                sftp_internal::download_sftp_internal_file(sftp_context, &state.ingestion).await
            }
        }
    }
}

impl IngestionContext {
    pub async fn parse_payload(&self) -> ParseStageResult<TriggerData> {
        match self {
            Self::Adyen(adyen_context) => parse_adyen_payload(
                adyen_context.raw_payload.clone(),
                &adyen_context.headers,
                adyen_context
                    .decrypted_config
                    .hmac_secret
                    .get_inner()
                    .peek(),
                &adyen_context.decrypted_config.webhook_basic_auth,
            ),
            Self::Manual { file_name, .. } => {
                Ok(TriggerData::V1(TriggerDataV1::Manual(ManualTriggerData {
                    file_name: file_name.clone(),
                })))
            }
            Self::SftpInternal(context) => {
                sftp_internal::authenticate_sftp_internal(context)?;
                Ok(TriggerData::V1(TriggerDataV1::SftpInternal))
            }
        }
    }

    pub fn get_raw_payload(&self) -> RawPayload {
        match self {
            Self::Adyen(adyen_context) => {
                RawPayload::V1(RawPayloadV1::Adyen(adyen_context.raw_payload.clone()))
            }
            Self::Manual { file_name, .. } => RawPayload::V1(RawPayloadV1::Manual {
                file_name: file_name.clone(),
            }),
            Self::SftpInternal { .. } => RawPayload::V1(RawPayloadV1::SftpInternal),
        }
    }

    pub async fn create_download_context(self) -> IngestionResult<FileDownloadContext> {
        match self {
            Self::Adyen(adyen_context) => {
                let trigger_data = parse_adyen_payload(
                    adyen_context.raw_payload,
                    &adyen_context.headers,
                    adyen_context
                        .decrypted_config
                        .hmac_secret
                        .get_inner()
                        .peek(),
                    &adyen_context.decrypted_config.webhook_basic_auth,
                )
                .switch()?;

                match trigger_data {
                    TriggerData::V1(TriggerDataV1::Adyen(adyen_data)) => Ok(
                        FileDownloadContext::Adyen(adyen::AdyenFileDownloadContext {
                            trigger_data: adyen_data,
                            report_basic_auth: adyen_context.decrypted_config.report_basic_auth,
                        }),
                    ),
                    _ => Err(ParseStageError::InvalidFormat {
                        reason: "Unexpected trigger data type for Adyen context".to_string(),
                    }
                    .into())
                    .switch(),
                }
            }
            Self::Manual {
                file_data,
                file_name,
            } => Ok(FileDownloadContext::Manual(
                manual::ManualFileDownloadContext {
                    file_data,
                    file_name,
                },
            )),
            Self::SftpInternal(context) => Ok(FileDownloadContext::SftpInternal(
                sftp_internal::SftpInternalFileDownloadContext {
                    file_path: context.config.file_path.clone(),
                },
            )),
        }
    }
}

/// Trait for parsing raw payload data into TriggerData
pub trait IngestionConfigDataExt {
    /// Create an IngestionContext from webhook data
    async fn create_context_from_webhook(
        &self,
        raw_payload: serde_json::Value,
        headers: HeaderMap,
        internal_api_key: Secret<String>,
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    ) -> IngestionResult<IngestionContext>;

    /// Create an IngestionContext for manual file uploads
    fn create_context_for_manual_upload(
        &self,
        file_data: Vec<u8>,
        file_name: String,
    ) -> IngestionResult<IngestionContext>;
}

impl IngestionConfigDataExt for IngestionConfigData {
    async fn create_context_from_webhook(
        &self,
        raw_payload: serde_json::Value,
        headers: HeaderMap,
        internal_api_key: Secret<String>,
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    ) -> IngestionResult<IngestionContext> {
        match self {
            Self::V1(IngestionConfigV1::Psp(IngestionConfigPsp::Adyen(diesel_config))) => {
                let domain_config = diesel_config
                    .clone()
                    .convert_back(encryptor)
                    .await
                    .change_context(IngestionError::ValidationError(
                        "Failed to decrypt Adyen config".to_string(),
                    ))?;

                Ok(IngestionContext::Adyen(Box::new(
                    adyen::AdyenIngestionContext {
                        decrypted_config: domain_config,
                        raw_payload,
                        headers,
                    },
                )))
            }
            Self::V1(IngestionConfigV1::Manual) => {
                Err(ParseStageError::WebhookNotSupported.into()).switch()
            }
            Self::V1(IngestionConfigV1::SftpInternal(diesel_config)) => {
                let domain_config = DomainSftpInternalIngestionConfig {
                    file_path: diesel_config.file_path.clone(),
                };

                Ok(IngestionContext::SftpInternal(Box::new(
                    sftp_internal::SftpInternalIngestionContext {
                        config: domain_config,
                        headers,
                        internal_api_key,
                    },
                )))
            }
        }
    }

    fn create_context_for_manual_upload(
        &self,
        file_data: Vec<u8>,
        file_name: String,
    ) -> IngestionResult<IngestionContext> {
        match self {
            Self::V1(IngestionConfigV1::Psp(_)) => {
                Err(ParseStageError::ManualUploadNotSupported.into()).switch()
            }
            Self::V1(IngestionConfigV1::Manual) => Ok(IngestionContext::Manual {
                file_data,
                file_name,
            }),
            Self::V1(IngestionConfigV1::SftpInternal(_)) => {
                Err(ParseStageError::ManualUploadNotSupported.into()).switch()
            }
        }
    }
}
