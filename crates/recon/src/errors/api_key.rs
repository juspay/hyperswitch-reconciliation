use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;
use error_stack::Report;
use thiserror::Error;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum ApiKeyError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("API key not found")]
    ApiKeyNotFound,

    #[error("name must not be empty")]
    NameEmpty,

    #[error("name must be at most {max_length} characters")]
    NameTooLong { max_length: usize },

    #[error("description must be at most {max_length} characters")]
    DescriptionTooLong { max_length: usize },

    #[error("expires_at must be in the future")]
    ExpiryInPast,

    #[error("expires_at must be within {max_days} days of creation")]
    ExpiryTooFar { max_days: i64 },

    #[error("status can only be updated to revoked")]
    InvalidStatusUpdate,

    #[error("at least one of name, description or status is required")]
    NothingToUpdate,
}

pub type ApiKeyResponse<T> = Result<ApplicationResponse<T>, Report<ApiKeyError>>;
pub type ApiKeyResult<T> = Result<T, Report<ApiKeyError>>;

impl From<ApiKeyError> for ReconAppError {
    fn from(error: ApiKeyError) -> Self {
        match error {
            ApiKeyError::InternalServerError => Self::InternalServerError,
            ApiKeyError::ApiKeyNotFound => Self::ResourceNotFound(error.to_string()),
            ApiKeyError::NameEmpty
            | ApiKeyError::NameTooLong { .. }
            | ApiKeyError::DescriptionTooLong { .. }
            | ApiKeyError::ExpiryInPast
            | ApiKeyError::ExpiryTooFar { .. }
            | ApiKeyError::InvalidStatusUpdate
            | ApiKeyError::NothingToUpdate => Self::ValidationError(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<ApiKeyError> for ApiErrorPayload {
    fn switch_from(error: &ApiKeyError) -> Self {
        let code = match error {
            ApiKeyError::InternalServerError => 1700,
            ApiKeyError::ApiKeyNotFound => 1701,
            ApiKeyError::NameEmpty => 1702,
            ApiKeyError::NameTooLong { .. } => 1703,
            ApiKeyError::DescriptionTooLong { .. } => 1704,
            ApiKeyError::ExpiryInPast => 1705,
            ApiKeyError::ExpiryTooFar { .. } => 1706,
            ApiKeyError::InvalidStatusUpdate => 1707,
            ApiKeyError::NothingToUpdate => 1708,
        };
        match error {
            ApiKeyError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                code,
                "An internal error occurred while processing API key request",
                None,
            )),
            ApiKeyError::ApiKeyNotFound => {
                Self::NotFound(ApiError::new("RE", code, "API key not found", None))
            }
            ApiKeyError::NameEmpty
            | ApiKeyError::NameTooLong { .. }
            | ApiKeyError::DescriptionTooLong { .. }
            | ApiKeyError::ExpiryInPast
            | ApiKeyError::ExpiryTooFar { .. }
            | ApiKeyError::InvalidStatusUpdate
            | ApiKeyError::NothingToUpdate => {
                Self::BadRequest(ApiError::new("RE", code, error.to_string(), None))
            }
        }
    }
}
