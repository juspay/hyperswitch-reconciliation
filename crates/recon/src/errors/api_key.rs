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

    #[error("{message}")]
    InvalidRequest { message: String },
}

pub type ApiKeyResult<T> = Result<ApplicationResponse<T>, Report<ApiKeyError>>;

impl From<ApiKeyError> for ReconAppError {
    fn from(error: ApiKeyError) -> Self {
        match error {
            ApiKeyError::InternalServerError => Self::InternalServerError,
            ApiKeyError::ApiKeyNotFound => Self::ResourceNotFound(error.to_string()),
            ApiKeyError::InvalidRequest { .. } => Self::ValidationError(error.to_string()),
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
            ApiKeyError::InvalidRequest { message } => {
                Self::BadRequest(ApiError::new("RE", 1703, message, None))
            }
        }
    }
}
