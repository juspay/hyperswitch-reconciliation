use common_utils::errors::ErrorSwitchFrom;
use error_stack::Report;
use thiserror::Error;

use crate::{errors::ReconAppError, types::ApplicationResponse};
use api_models::{
    entry::EntryListFilterValidationError,
    errors::{ApiError, ApiErrorResponse as ApiErrorPayload},
};

#[derive(Debug, Clone, Error)]
pub enum EntryError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Entry not found")]
    EntryNotFound,

    #[error("Account not found: {account_id}")]
    AccountNotFound { account_id: String },

    #[error("Invalid amount: {details}")]
    InvalidAmount { details: String },

    #[error("Start time cannot be greater than End Time")]
    TimeRangeValidationError,

    #[error("Transaction not found: {id}")]
    TransactionNotFound { id: String },
}

pub type EntryResponse<T> = Result<ApplicationResponse<T>, Report<EntryError>>;
pub type EntryResult<T> = Result<T, Report<EntryError>>;

impl From<EntryError> for ReconAppError {
    fn from(value: EntryError) -> Self {
        match value {
            EntryError::InternalServerError => Self::InternalServerError,
            EntryError::EntryNotFound => Self::ResourceNotFound(value.to_string()),
            EntryError::AccountNotFound { .. } => Self::ResourceNotFound(value.to_string()),
            EntryError::InvalidAmount { .. } => Self::ValidationError(value.to_string()),
            EntryError::TimeRangeValidationError => Self::ValidationError(value.to_string()),
            EntryError::TransactionNotFound { .. } => Self::ResourceNotFound(value.to_string()),
        }
    }
}

impl ErrorSwitchFrom<EntryError> for ApiErrorPayload {
    fn switch_from(error: &EntryError) -> Self {
        match error {
            EntryError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                500,
                "An internal error occurred while processing entry request",
                None,
            )),
            EntryError::EntryNotFound => {
                Self::NotFound(ApiError::new("RE", 501, "Entry not found", None))
            }
            EntryError::InvalidAmount { details } => Self::BadRequest(ApiError::new(
                "RE",
                502,
                format!("Invalid amount format: {details}"),
                None,
            )),
            EntryError::AccountNotFound { account_id } => Self::NotFound(ApiError::new(
                "RE",
                503,
                format!("Account not found: {account_id}"),
                None,
            )),
            EntryError::TimeRangeValidationError => {
                Self::BadRequest(ApiError::new("RE", 504, error.to_string(), None))
            }
            EntryError::TransactionNotFound { id } => Self::NotFound(ApiError::new(
                "RE",
                505,
                format!("Transaction not found: {id}"),
                None,
            )),
        }
    }
}

impl ErrorSwitchFrom<EntryListFilterValidationError> for EntryError {
    fn switch_from(error: &EntryListFilterValidationError) -> Self {
        match error {
            EntryListFilterValidationError::InvalidTimeRange => Self::TimeRangeValidationError,
        }
    }
}
