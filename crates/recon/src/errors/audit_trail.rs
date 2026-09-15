use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Error)]
pub enum AuditTrailError {
    #[error("An internal error occurred")]
    InternalServerError,
    #[error("Invalid time range: start_time cannot be greater than end_time")]
    InvalidTimeRange,
}

// Type alias for function signatures
pub type AuditTrailResponse<T> = Result<ApplicationResponse<T>, Report<AuditTrailError>>;

impl From<AuditTrailError> for ReconAppError {
    fn from(error: AuditTrailError) -> Self {
        match error {
            AuditTrailError::InternalServerError => Self::InternalServerError,
            AuditTrailError::InvalidTimeRange => Self::ValidationError(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<AuditTrailError> for ApiErrorPayload {
    fn switch_from(error: &AuditTrailError) -> Self {
        match error {
            AuditTrailError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                1200,
                "An internal error occurred",
                None,
            )),
            AuditTrailError::InvalidTimeRange => Self::BadRequest(ApiError::new(
                "RE",
                1201,
                "Invalid time range: start_time cannot be greater than end_time",
                None,
            )),
        }
    }
}
