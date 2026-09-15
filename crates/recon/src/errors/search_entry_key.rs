use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;
use error_stack::Report;
use thiserror::Error;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum SearchEntryError {
    #[error("An internal error occurred")]
    InternalServerError,
}

pub type SearchEntryResult<T> = Result<ApplicationResponse<T>, Report<SearchEntryError>>;

impl From<SearchEntryError> for ReconAppError {
    fn from(error: SearchEntryError) -> Self {
        match error {
            SearchEntryError::InternalServerError => Self::InternalServerError,
        }
    }
}

impl ErrorSwitchFrom<SearchEntryError> for ApiErrorPayload {
    fn switch_from(error: &SearchEntryError) -> Self {
        match error {
            SearchEntryError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                1700,
                "An internal error occurred while processing search entry key request",
                None,
            )),
        }
    }
}
