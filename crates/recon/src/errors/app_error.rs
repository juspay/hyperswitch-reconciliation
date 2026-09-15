use actix_web::ResponseError;
use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::{ErrorSwitch, ErrorSwitchFrom};
use external_services::user::types::error::UserClientError;
use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ReconAppError {
    #[error("An unexpected error occurred. Please try again later.")]
    InternalServerError,
    #[error("The requested URL was not found.")]
    InvalidRequestUrl,
    #[error("The HTTP method is not applicable for this URL.")]
    InvalidHttpMethod,
    #[error("{0}")]
    ResourceNotFound(String),
    #[error("{0}")]
    ResourceConflict(String),
    #[error("{0}")]
    ValidationError(String),
    #[error("Authentication required")]
    Unauthorized,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error("Not authorized to access this resource")]
    Forbidden,
}

impl ErrorSwitchFrom<ReconAppError> for ApiErrorPayload {
    fn switch_from(error: &ReconAppError) -> Self {
        match error {
            ReconAppError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                1,
                "An unexpected error occurred. Please try again later.",
                None,
            )),
            ReconAppError::InvalidRequestUrl => Self::NotFound(ApiError::new(
                "RE",
                2,
                "The requested URL was not found on this server.",
                None,
            )),
            ReconAppError::InvalidHttpMethod => Self::MethodNotAllowed(ApiError::new(
                "RE",
                3,
                "The HTTP method is not supported for this URL.",
                None,
            )),
            ReconAppError::ResourceNotFound(message) => {
                Self::NotFound(ApiError::new("RE", 4, message, None))
            }
            ReconAppError::ResourceConflict(message) => {
                Self::Conflict(ApiError::new("RE", 5, message, None))
            }
            ReconAppError::ValidationError(message) => {
                Self::BadRequest(ApiError::new("RE", 6, message, None))
            }
            ReconAppError::Unauthorized => {
                Self::Unauthorized(ApiError::new("RE", 7, "Authentication required", None))
            }
            ReconAppError::InvalidToken => {
                Self::Unauthorized(ApiError::new("RE", 8, "Invalid or expired token", None))
            }
            ReconAppError::Forbidden => Self::ForbiddenCommonResource(ApiError::new(
                "RE",
                9,
                "Not authorized to access this resource",
                None,
            )),
        }
    }
}

impl ErrorSwitchFrom<UserClientError> for ReconAppError {
    fn switch_from(error: &UserClientError) -> Self {
        match error {
            UserClientError::Unauthorized => Self::InvalidToken,
            UserClientError::Forbidden => Self::Forbidden,
            UserClientError::InternalServerError => Self::InternalServerError,
        }
    }
}

impl ResponseError for ReconAppError {
    fn status_code(&self) -> StatusCode {
        <Self as ErrorSwitch<ApiErrorPayload>>::switch(self).status_code()
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        <Self as ErrorSwitch<ApiErrorPayload>>::switch(self).error_response()
    }
}
