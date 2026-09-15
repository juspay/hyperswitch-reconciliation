use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::types::ApplicationResponse;

#[derive(Debug, Error)]
pub enum BulkOperationError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Bulk operation exceeds record limit: max {max}")]
    RecordLimitExceeded { max: usize },
}

pub type BulkOperationResult<T> = Result<T, Report<BulkOperationError>>;
pub type BulkOperationResponse<T> = Result<ApplicationResponse<T>, Report<BulkOperationError>>;

impl ErrorSwitchFrom<BulkOperationError> for ApiErrorPayload {
    fn switch_from(error: &BulkOperationError) -> Self {
        match error {
            BulkOperationError::InternalServerError => {
                Self::InternalServerError(ApiError::new("RE", 1400, error.to_string(), None))
            }
            BulkOperationError::RecordLimitExceeded { .. } => {
                Self::BadRequest(ApiError::new("RE", 1401, error.to_string(), None))
            }
        }
    }
}
