use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Invalid account ID provided")]
    InvalidAccountId,

    #[error("Account not found: {account_id}")]
    AccountNotFound { account_id: String },

    #[error("Account name cannot be empty")]
    AccountNameEmpty,

    #[error("Invalid amount provided")]
    InvalidAmount,
}

// Type alias for function signatures
pub type AccountResponse<T> = Result<ApplicationResponse<T>, Report<AccountError>>;
pub type AccountResult<T> = Result<T, Report<AccountError>>;

impl From<AccountError> for ReconAppError {
    fn from(error: AccountError) -> Self {
        match error {
            AccountError::InternalServerError => Self::InternalServerError,
            AccountError::InvalidAccountId => Self::ValidationError(error.to_string()),
            AccountError::AccountNotFound { .. } => Self::ResourceNotFound(error.to_string()),
            AccountError::AccountNameEmpty => Self::ValidationError(error.to_string()),
            AccountError::InvalidAmount => Self::ValidationError(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<AccountError> for ApiErrorPayload {
    fn switch_from(error: &AccountError) -> Self {
        match error {
            AccountError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                300,
                "An internal error occurred while processing account request",
                None,
            )),
            AccountError::InvalidAccountId => Self::BadRequest(ApiError::new(
                "RE",
                301,
                "Invalid account ID provided",
                None,
            )),
            AccountError::AccountNotFound { account_id } => Self::NotFound(ApiError::new(
                "RE",
                302,
                format!("Account not found: {account_id}"),
                None,
            )),
            AccountError::AccountNameEmpty => Self::BadRequest(ApiError::new(
                "RE",
                304,
                "Account name cannot be empty",
                None,
            )),
            AccountError::InvalidAmount => {
                Self::BadRequest(ApiError::new("RE", 305, "Invalid amount provided", None))
            }
        }
    }
}
