use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;
use error_stack::Report;
use thiserror::Error;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum SearchExpectationError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Transaction not found")]
    TransactionNotFound,
}

pub type SearchExpectationResult<T> =
    Result<ApplicationResponse<T>, Report<SearchExpectationError>>;

impl From<SearchExpectationError> for ReconAppError {
    fn from(error: SearchExpectationError) -> Self {
        match error {
            SearchExpectationError::InternalServerError => Self::InternalServerError,
            SearchExpectationError::TransactionNotFound => Self::ValidationError(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<SearchExpectationError> for ApiErrorPayload {
    fn switch_from(error: &SearchExpectationError) -> Self {
        match error {
            SearchExpectationError::InternalServerError => {
                Self::InternalServerError(ApiError::new(
                    "RE",
                    1500,
                    "An internal error occurred while processing search expectation request",
                    None,
                ))
            }
            SearchExpectationError::TransactionNotFound => {
                Self::NotFound(ApiError::new("RE", 1501, "Transaction not found", None))
            }
        }
    }
}
