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

    #[error("at least one of name or description is required")]
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
            | ApiKeyError::NothingToUpdate => Self::ValidationError(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<ApiKeyError> for ApiErrorPayload {
    fn switch_from(error: &ApiKeyError) -> Self {
        match error {
            ApiKeyError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                1700,
                "An internal error occurred while processing API key request",
                None,
            )),
            ApiKeyError::ApiKeyNotFound => {
                Self::NotFound(ApiError::new("RE", 1701, "API key not found", None))
            }
            ApiKeyError::NameEmpty => {
                Self::BadRequest(ApiError::new("RE", 1702, error.to_string(), None))
            }
            ApiKeyError::NameTooLong { .. } => {
                Self::BadRequest(ApiError::new("RE", 1703, error.to_string(), None))
            }
            ApiKeyError::DescriptionTooLong { .. } => {
                Self::BadRequest(ApiError::new("RE", 1704, error.to_string(), None))
            }
            ApiKeyError::ExpiryInPast => {
                Self::BadRequest(ApiError::new("RE", 1705, error.to_string(), None))
            }
            ApiKeyError::NothingToUpdate => {
                Self::BadRequest(ApiError::new("RE", 1708, error.to_string(), None))
            }
        }
    }
}
