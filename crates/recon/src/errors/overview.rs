use common_utils::errors::ErrorSwitchFrom;
use error_stack::Report;
use thiserror::Error;

use crate::{errors::ReconAppError, types::ApplicationResponse};
use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};

#[derive(Debug, Clone, Error)]
pub enum OverviewError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Start time cannot be greater than End Time")]
    TimeRangeValidationError,

    #[error("Requested time range and granularity would generate too many buckets")]
    TooManyBucketsError,
}

pub type OverviewResponse<T> = Result<ApplicationResponse<T>, Report<OverviewError>>;
pub type OverviewResult<T> = Result<T, Report<OverviewError>>;

impl From<OverviewError> for ReconAppError {
    fn from(value: OverviewError) -> Self {
        match value {
            OverviewError::InternalServerError => Self::InternalServerError,
            OverviewError::TimeRangeValidationError => Self::ValidationError(value.to_string()),
            OverviewError::TooManyBucketsError => Self::ValidationError(value.to_string()),
        }
    }
}

impl ErrorSwitchFrom<OverviewError> for ApiErrorPayload {
    fn switch_from(error: &OverviewError) -> Self {
        match error {
            OverviewError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                1600,
                "An internal error occurred while processing overview request",
                None,
            )),
            OverviewError::TimeRangeValidationError => {
                Self::BadRequest(ApiError::new("RE", 1601, error.to_string(), None))
            }
            OverviewError::TooManyBucketsError => {
                Self::BadRequest(ApiError::new("RE", 1602, error.to_string(), None))
            }
        }
    }
}
