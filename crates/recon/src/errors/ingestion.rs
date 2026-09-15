use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::types::ApplicationResponse;

#[derive(Debug, Error)]
pub enum IngestionError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Ingestion configuration not found: {ingestion_id}")]
    IngestionConfigNotFound { ingestion_id: String },

    #[error("Ingestion is not active")]
    IngestionNotActive,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("Profile not found: {profile_id}")]
    ProfileNotFound { profile_id: String },

    #[error("Account not found or does not belong to profile: {account_id}")]
    AccountNotFound { account_id: String },

    #[error("Ingestion history not found: {history_id}")]
    IngestionHistoryNotFound { history_id: String },

    #[error("File not available for download")]
    FileNotAvailable,
}

#[derive(Debug, Error)]
pub enum ParseStageError {
    #[error("Failed to parse JSON: {reason}")]
    JsonParseError { reason: String },

    #[error("Manual ingestion does not support parsing")]
    ManualIngestionNotSupported,

    #[error("Webhook triggers not supported for this configuration")]
    WebhookNotSupported,

    #[error("Manual file upload not supported for this configuration")]
    ManualUploadNotSupported,

    #[error("Invalid authentication: {reason}")]
    InvalidAuthentication { reason: String },

    #[error("Invalid HMAC signature")]
    InvalidHmacSignature,

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid data format: {reason}")]
    InvalidFormat { reason: String },

    #[error("Time range validation failed")]
    TimeRangeValidation,
}

#[derive(Debug, Error)]
pub enum DownloadStageError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("HTTP client error")]
    HttpClientError { reason: String },

    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("External service error: {message}")]
    ExternalServiceError { message: String },

    #[error("Failed to read response")]
    ResponseReadError,

    #[error("Invalid download URL: {url}")]
    InvalidUrl { url: String },

    #[error("Report not available: {reason}")]
    ReportNotAvailable { reason: String },
}

#[derive(Debug, Error)]
pub enum StorageStageError {
    #[error("Failed to upload file: {reason}")]
    UploadFailed { reason: String },
}

// Type alias for function signatures
pub type IngestionResponse<T> = Result<ApplicationResponse<T>, Report<IngestionError>>;
pub type IngestionResult<T> = Result<T, Report<IngestionError>>;
pub type ParseStageResult<T> = Result<T, Report<ParseStageError>>;
pub type DownloadStageResult<T> = Result<T, Report<DownloadStageError>>;
pub type StorageStageResult<T> = Result<T, Report<StorageStageError>>;

// ErrorSwitch implementations for stage errors to IngestionError
impl ErrorSwitchFrom<ParseStageError> for IngestionError {
    fn switch_from(error: &ParseStageError) -> Self {
        match error {
            ParseStageError::JsonParseError { .. }
            | ParseStageError::InvalidHmacSignature
            | ParseStageError::MissingField { .. }
            | ParseStageError::InvalidAuthentication { .. }
            | ParseStageError::InvalidFormat { .. }
            | ParseStageError::TimeRangeValidation => Self::ValidationError(error.to_string()),
            ParseStageError::ManualIngestionNotSupported
            | ParseStageError::WebhookNotSupported
            | ParseStageError::ManualUploadNotSupported => Self::NotSupported(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<DownloadStageError> for IngestionError {
    fn switch_from(error: &DownloadStageError) -> Self {
        match error {
            DownloadStageError::InternalServerError => Self::InternalServerError,
            DownloadStageError::AuthenticationFailed { .. } => {
                Self::ValidationError(format!("Configuration error: {error}"))
            }
            DownloadStageError::InvalidUrl { .. } => Self::ValidationError(error.to_string()),
            DownloadStageError::ExternalServiceError { .. }
            | DownloadStageError::ReportNotAvailable { .. }
            | DownloadStageError::HttpClientError { .. }
            | DownloadStageError::ResponseReadError => {
                Self::ExternalServiceError(error.to_string())
            }
        }
    }
}

impl ErrorSwitchFrom<StorageStageError> for IngestionError {
    fn switch_from(error: &StorageStageError) -> Self {
        match error {
            StorageStageError::UploadFailed { .. } => Self::InternalServerError,
        }
    }
}

impl ErrorSwitchFrom<IngestionError> for ApiErrorPayload {
    fn switch_from(error: &IngestionError) -> Self {
        match error {
            IngestionError::InternalServerError => {
                Self::InternalServerError(ApiError::new("IE", 900, error.to_string(), None))
            }
            IngestionError::IngestionConfigNotFound { .. } => {
                Self::NotFound(ApiError::new("IE", 901, error.to_string(), None))
            }
            IngestionError::IngestionNotActive => {
                Self::BadRequest(ApiError::new("IE", 902, error.to_string(), None))
            }
            IngestionError::ValidationError(_) => {
                Self::BadRequest(ApiError::new("IE", 903, error.to_string(), None))
            }
            IngestionError::ExternalServiceError(_) => {
                Self::BadGateway(ApiError::new("IE", 904, error.to_string(), None))
            }
            IngestionError::NotSupported(_) => {
                Self::BadRequest(ApiError::new("IE", 905, error.to_string(), None))
            }
            IngestionError::ProfileNotFound { .. } => {
                Self::BadRequest(ApiError::new("IE", 906, error.to_string(), None))
            }
            IngestionError::AccountNotFound { .. } => {
                Self::BadRequest(ApiError::new("IE", 907, error.to_string(), None))
            }
            IngestionError::IngestionHistoryNotFound { .. } => {
                Self::NotFound(ApiError::new("IE", 908, error.to_string(), None))
            }
            IngestionError::FileNotAvailable => {
                Self::NotFound(ApiError::new("IE", 909, error.to_string(), None))
            }
        }
    }
}
