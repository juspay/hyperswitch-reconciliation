use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum MerchantAccountError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Merchant name cannot be empty")]
    MerchantNameEmpty,

    #[error("Merchant ID cannot be empty")]
    MerchantIdEmpty,
}

// Type alias for cleaner function signatures
pub type MerchantAccountResult<T> = Result<ApplicationResponse<T>, Report<MerchantAccountError>>;

impl From<MerchantAccountError> for ReconAppError {
    fn from(error: MerchantAccountError) -> Self {
        match error {
            MerchantAccountError::InternalServerError => Self::InternalServerError,
            MerchantAccountError::MerchantNameEmpty => Self::ValidationError(error.to_string()),
            MerchantAccountError::MerchantIdEmpty => Self::ValidationError(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<MerchantAccountError> for ApiErrorPayload {
    fn switch_from(error: &MerchantAccountError) -> Self {
        match error {
            MerchantAccountError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                100,
                "An internal error occurred while processing merchant account request",
                None,
            )),
            MerchantAccountError::MerchantNameEmpty => Self::BadRequest(ApiError::new(
                "RE",
                101,
                "Merchant name cannot be empty",
                None,
            )),
            MerchantAccountError::MerchantIdEmpty => Self::BadRequest(ApiError::new(
                "RE",
                102,
                "Merchant ID cannot be empty",
                None,
            )),
        }
    }
}
