use std::path::PathBuf;

use error_stack::{report, ResultExt};
use recon_domain_models::ingestion::sftp_internal::SftpInternalIngestionConfig;

use crate::{
    domain::ingestion::FileDownloadResult,
    errors::ingestion::{DownloadStageError, DownloadStageResult},
    routes::IngestionState,
};

#[derive(Clone, Debug)]
pub struct SftpInternalFileDownloadContext {
    pub file_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct SftpInternalIngestionContext {
    pub config: SftpInternalIngestionConfig,
}

/// Download a file from S3 using the SFTP Internal configuration
pub async fn download_sftp_internal_file(
    context: &SftpInternalFileDownloadContext,
    state: &IngestionState,
) -> DownloadStageResult<FileDownloadResult> {
    // Convert PathBuf to string for S3 key
    let file_key = context
        .file_path
        .to_str()
        .ok_or(DownloadStageError::InternalServerError)?;

    // Retrieve file from S3 using the SFTP-specific file storage handler
    let file_content = state
        .sftp_file_storage
        .retrieve_file(file_key)
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
