use std::path::PathBuf;

use actix_web::http::header::HeaderMap;
use common_utils::id_type::MerchantId;
use error_stack::{report, ResultExt};
use masking::{PeekInterface, Secret};
use recon_domain_models::ingestion::sftp_internal::SftpInternalIngestionConfig;

use crate::{
    domain::ingestion::FileDownloadResult,
    errors::ingestion::{
        DownloadStageError, DownloadStageResult, ParseStageError, ParseStageResult,
    },
    routes::IngestionState,
    services::auth::get_internal_api_key,
};

#[derive(Clone, Debug)]
pub struct SftpInternalFileDownloadContext {
    pub file_path: PathBuf,
    pub merchant_id: MerchantId,
}

#[derive(Clone, Debug)]
pub struct SftpInternalIngestionContext {
    pub config: SftpInternalIngestionConfig,
    pub merchant_id: MerchantId,
    pub headers: HeaderMap,
    pub internal_api_key: Secret<String>,
}

pub fn authenticate_sftp_internal(context: &SftpInternalIngestionContext) -> ParseStageResult<()> {
    let invalid_authentication = || ParseStageError::InvalidAuthentication {
        reason: "Missing or invalid internal API key".to_string(),
    };

    let request_internal_api_key =
        get_internal_api_key(&context.headers).change_context(invalid_authentication())?;

    if request_internal_api_key != context.internal_api_key.peek() {
        return Err(report!(invalid_authentication()));
    }

    Ok(())
}

/// Download a file from S3 using the SFTP Internal configuration
pub async fn download_sftp_internal_file(
    context: &SftpInternalFileDownloadContext,
    state: &IngestionState,
) -> DownloadStageResult<FileDownloadResult> {
    let relative_path = context
        .file_path
        .to_str()
        .ok_or(DownloadStageError::InternalServerError)?;
    let file_key = format!(
        "sftp/{}/{relative_path}",
        context.merchant_id.get_string_repr()
    );

    // Retrieve file from S3 using the SFTP-specific file storage handler
    let file_content = state
        .sftp_file_storage
        .retrieve_file(&file_key)
        .await
        .change_context(DownloadStageError::ExternalServiceError {
            message: format!("Failed to retrieve file from S3: {file_key}"),
        })?;

    // Extract filename from the path
    let file_name = context
        .file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            report!(DownloadStageError::InvalidUrl {
                url: file_key.to_string(),
            })
            .attach_printable("Could not extract filename from path")
        })?
        .to_string();

    Ok(FileDownloadResult {
        file_content,
        file_name,
    })
}
