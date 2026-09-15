use common_utils::errors::ErrorSwitch;
use error_stack::Report;

use crate::types::ApplicationResponse;

#[derive(Debug, thiserror::Error)]
pub enum TransformationError {
    #[error("Internal server error")]
    InternalServerError,

    #[error("Transformation config not found for ingestion_id: {ingestion_id}")]
    ConfigNotFound { ingestion_id: String },

    #[error("Invalid file format: {details}")]
    InvalidFileFormat { details: String },

    #[error("Transformation config not found: {id}")]
    TransformationConfigNotFound { id: String },

    #[error("Start time cannot be greater than End Time")]
    TimeRangeValidationError,

    #[error("Invalid file name")]
    InvalidFileName,

    #[error("Profile not found: {profile_id}")]
    ProfileNotFound { profile_id: String },

    #[error("Account not found or does not belong to profile: {account_id}")]
    AccountNotFound { account_id: String },

    #[error("Metadata schema not found: {metadata_schema_id}")]
    MetadataSchemaNotFound { metadata_schema_id: String },

    #[error("Ingestion configuration not found: {ingestion_id}")]
    IngestionConfigNotFound { ingestion_id: String },

    #[error("Metadata schema not found for transformation: {transformation_id}")]
    MetadataSchemaNotFoundForTransformation { transformation_id: String },

    #[error("No conditions for skip config found in request")]
    EmptySkipConfigConditionsFound,

    #[error("Transformation config name cannot be empty")]
    InvalidName,

    #[error("Transformation history not found: {id}")]
    TransformationHistoryNotFound { id: String },

    #[error("Transformation report is not available")]
    ReportNotAvailable,

    #[error("Failed to generate the transformation report")]
    ReportGenerationFailed,

    #[error("Failed to upload the transformation report")]
    ReportUploadFailed,
}

impl ErrorSwitch<api_models::errors::ApiErrorResponse> for TransformationError {
    fn switch(&self) -> api_models::errors::ApiErrorResponse {
        use api_models::errors::{ApiError, ApiErrorResponse};

        match self {
            Self::InternalServerError => ApiErrorResponse::InternalServerError(ApiError::new(
                "TR",
                1000,
                self.to_string(),
                None,
            )),
            Self::ConfigNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("TR", 1001, self.to_string(), None))
            }
            Self::InvalidFileFormat { .. } => {
                ApiErrorResponse::BadRequest(ApiError::new("TR", 1002, self.to_string(), None))
            }
            Self::TransformationConfigNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("TR", 1003, self.to_string(), None))
            }
            Self::TimeRangeValidationError => {
                ApiErrorResponse::BadRequest(ApiError::new("TR", 1004, self.to_string(), None))
            }
            Self::InvalidFileName => {
                ApiErrorResponse::BadRequest(ApiError::new("TR", 1005, self.to_string(), None))
            }
            Self::ProfileNotFound { .. } => {
                ApiErrorResponse::BadRequest(ApiError::new("TR", 1006, self.to_string(), None))
            }
            Self::AccountNotFound { .. } => {
                ApiErrorResponse::BadRequest(ApiError::new("TR", 1007, self.to_string(), None))
            }
            Self::MetadataSchemaNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("TR", 1008, self.to_string(), None))
            }
            Self::IngestionConfigNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("TR", 1009, self.to_string(), None))
            }
            Self::MetadataSchemaNotFoundForTransformation { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("TR", 1010, self.to_string(), None))
            }
            Self::EmptySkipConfigConditionsFound => {
                ApiErrorResponse::BadRequest(ApiError::new("TR", 1011, self.to_string(), None))
            }
            Self::InvalidName => {
                ApiErrorResponse::BadRequest(ApiError::new("TR", 1012, self.to_string(), None))
            }
            Self::TransformationHistoryNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("TR", 1013, self.to_string(), None))
            }
            Self::ReportNotAvailable => {
                ApiErrorResponse::NotFound(ApiError::new("TR", 1014, self.to_string(), None))
            }
            Self::ReportGenerationFailed => ApiErrorResponse::InternalServerError(ApiError::new(
                "TR",
                1015,
                self.to_string(),
                None,
            )),
            Self::ReportUploadFailed => ApiErrorResponse::InternalServerError(ApiError::new(
                "TR",
                1016,
                self.to_string(),
                None,
            )),
        }
    }
}

pub type TransformationResponse<T> = Result<ApplicationResponse<T>, Report<TransformationError>>;
pub type TransformationResult<T> = Result<T, Report<TransformationError>>;
