use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::{
    domain::metadata_schema::types::ConfigValidationError, errors::ReconAppError,
    types::ApplicationResponse,
};

#[derive(Debug, Error)]
pub enum MetadataSchemaError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Metadata schema not found")]
    MetadataSchemaNotFound,

    #[error("Start time cannot be greater than End Time")]
    TimeRangeValidationError,

    #[error("Account not found")]
    AccountNotFound { account_id: String },

    #[error("Metadata configuration error")]
    MetadataConfigError(ConfigValidationError),
}

// Type alias for cleaner function signatures
pub type MetadataSchemaResult<T> = Result<ApplicationResponse<T>, Report<MetadataSchemaError>>;

impl From<MetadataSchemaError> for ReconAppError {
    fn from(error: MetadataSchemaError) -> Self {
        match error {
            MetadataSchemaError::InternalServerError => Self::InternalServerError,
            MetadataSchemaError::MetadataSchemaNotFound => {
                Self::ResourceNotFound(error.to_string())
            }
            MetadataSchemaError::TimeRangeValidationError => {
                Self::ValidationError(error.to_string())
            }
            MetadataSchemaError::AccountNotFound { .. } => {
                Self::ResourceNotFound(error.to_string())
            }
            MetadataSchemaError::MetadataConfigError(_) => Self::ValidationError(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<MetadataSchemaError> for ApiErrorPayload {
    fn switch_from(error: &MetadataSchemaError) -> Self {
        match error {
            MetadataSchemaError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                1100,
                "An internal error occurred while processing metadata schema request",
                None,
            )),
            MetadataSchemaError::MetadataSchemaNotFound => {
                Self::NotFound(ApiError::new("RE", 1101, "Metadata schema not found", None))
            }
            MetadataSchemaError::TimeRangeValidationError => Self::BadRequest(ApiError::new(
                "RE",
                1102,
                "Start time cannot be greater than End Time",
                None,
            )),
            MetadataSchemaError::AccountNotFound { account_id } => Self::NotFound(ApiError::new(
                "RE",
                1103,
                format!("Account not found : {account_id}"),
                None,
            )),
            MetadataSchemaError::MetadataConfigError(error) => Self::BadRequest(ApiError::new(
                "RE",
                1104,
                format!("Invalid metadata schema configuration: {error}"),
                None,
            )),
        }
    }
}
