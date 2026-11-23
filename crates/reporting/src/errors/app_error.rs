use actix_web::{http::StatusCode, ResponseError};
use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::{ErrorSwitch, ErrorSwitchFrom};
use thiserror::Error;

/// Error codes use the `RS` prefix so they can't be confused with recon's `RE` codes.
const ERROR_PREFIX: &str = "RS";

#[derive(Debug, Clone, Error)]
pub enum ReportingAppError {
    #[error("An unexpected error occurred. Please try again later.")]
    InternalServerError,
    #[error("The requested URL was not found.")]
    InvalidRequestUrl,
    #[error("The HTTP method is not applicable for this URL.")]
    InvalidHttpMethod,
    #[error("{0}")]
    ValidationError(String),
}

impl ErrorSwitchFrom<ReportingAppError> for ApiErrorPayload {
    fn switch_from(error: &ReportingAppError) -> Self {
        match error {
            ReportingAppError::InternalServerError => Self::InternalServerError(ApiError::new(
                ERROR_PREFIX,
                1,
                "An unexpected error occurred. Please try again later.",
                None,
            )),
            ReportingAppError::InvalidRequestUrl => Self::NotFound(ApiError::new(
                ERROR_PREFIX,
                2,
                "The requested URL was not found on this server.",
                None,
            )),
            ReportingAppError::InvalidHttpMethod => Self::MethodNotAllowed(ApiError::new(
                ERROR_PREFIX,
                3,
                "The HTTP method is not supported for this URL.",
                None,
            )),
            ReportingAppError::ValidationError(message) => {
                Self::BadRequest(ApiError::new(ERROR_PREFIX, 4, message, None))
            }
        }
    }
}

impl ResponseError for ReportingAppError {
    fn status_code(&self) -> StatusCode {
        <Self as ErrorSwitch<ApiErrorPayload>>::switch(self).status_code()
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        <Self as ErrorSwitch<ApiErrorPayload>>::switch(self).error_response()
    }
}
