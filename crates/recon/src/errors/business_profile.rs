use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum BusinessProfileError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Profile name cannot be empty")]
    ProfileNameEmpty,

    #[error("Profile ID cannot be empty")]
    ProfileIdEmpty,

    #[error("Business profile not found: {profile_id}")]
    ProfileNotFound { profile_id: String },
}

// Type alias for function signatures
pub type BusinessProfileResult<T> = Result<ApplicationResponse<T>, Report<BusinessProfileError>>;

impl From<BusinessProfileError> for ReconAppError {
    fn from(error: BusinessProfileError) -> Self {
        match error {
            BusinessProfileError::InternalServerError => Self::InternalServerError,
            BusinessProfileError::ProfileNameEmpty => Self::ValidationError(error.to_string()),
            BusinessProfileError::ProfileIdEmpty => Self::ValidationError(error.to_string()),
            BusinessProfileError::ProfileNotFound { .. } => {
                Self::ResourceNotFound(error.to_string())
            }
        }
    }
}

impl ErrorSwitchFrom<BusinessProfileError> for ApiErrorPayload {
    fn switch_from(error: &BusinessProfileError) -> Self {
        match error {
            BusinessProfileError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                200,
                "An internal error occurred while processing business profile request",
                None,
            )),
            BusinessProfileError::ProfileNameEmpty => Self::BadRequest(ApiError::new(
                "RE",
                201,
                "Profile name cannot be empty",
                None,
            )),
            BusinessProfileError::ProfileIdEmpty => {
                Self::BadRequest(ApiError::new("RE", 202, "Profile ID cannot be empty", None))
            }
            BusinessProfileError::ProfileNotFound { profile_id } => Self::NotFound(ApiError::new(
                "RE",
                203,
                format!("Business profile not found: {profile_id}"),
                None,
            )),
        }
    }
}
