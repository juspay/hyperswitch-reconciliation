use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::types::ApplicationResponse;

#[derive(Debug, Clone, Error)]
pub enum LinkingEngineError {
    #[error("An internal error occurred")]
    InternalServerError,
}

pub type LinkingEngineResult<T> = Result<T, Report<LinkingEngineError>>;
pub type LinkingEngineResponse<T> = Result<ApplicationResponse<T>, Report<LinkingEngineError>>;

impl ErrorSwitchFrom<LinkingEngineError> for ApiErrorPayload {
    fn switch_from(error: &LinkingEngineError) -> Self {
        match error {
            LinkingEngineError::InternalServerError => Self::InternalServerError(ApiError::new(
                "LE",
                800,
                "An internal error occurred",
                None,
            )),
        }
    }
}
