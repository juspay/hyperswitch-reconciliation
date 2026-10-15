use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::types::ApplicationResponse;

#[derive(Debug, Clone, Error)]
pub enum LinkingProcessorError {
    #[error("An internal error occurred")]
    InternalServerError,
}

pub type LinkingProcessorResult<T> = Result<T, Report<LinkingProcessorError>>;
pub type LinkingProcessorResponse<T> =
    Result<ApplicationResponse<T>, Report<LinkingProcessorError>>;

impl ErrorSwitchFrom<LinkingProcessorError> for ApiErrorPayload {
    fn switch_from(error: &LinkingProcessorError) -> Self {
        match error {
            LinkingProcessorError::InternalServerError => Self::InternalServerError(ApiError::new(
                "LE",
                800,
                "An internal error occurred",
                None,
            )),
        }
    }
}
