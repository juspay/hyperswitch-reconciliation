use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;
use error_stack::Report;
use thiserror::Error;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum SearchGroupError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Search Group not found")]
    SearchGroupNotFound,
}

pub type SearchGroupResult<T> = Result<ApplicationResponse<T>, Report<SearchGroupError>>;

impl From<SearchGroupError> for ReconAppError {
    fn from(error: SearchGroupError) -> Self {
        match error {
            SearchGroupError::InternalServerError => Self::InternalServerError,
            SearchGroupError::SearchGroupNotFound => Self::ValidationError(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<SearchGroupError> for ApiErrorPayload {
    fn switch_from(error: &SearchGroupError) -> Self {
        match error {
            SearchGroupError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                1300,
                "An internal error occurred while processing search group request",
                None,
            )),
            SearchGroupError::SearchGroupNotFound => {
                Self::NotFound(ApiError::new("RE", 1301, "Search group not found", None))
            }
        }
    }
}
