use api_models::errors::{ApiError, ApiErrorResponse};
use common_utils::errors::ErrorSwitch;
use error_stack::Report;

use crate::types::ApplicationResponse;

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("Internal server error")]
    InternalServerError,
    #[error("Invalid time range")]
    TimeRangeValidationError,
    #[error("Failed to serialize report payload")]
    PayloadSerializationFailed,
    #[error("Recon rule not found: {rule_id}")]
    RuleNotFound { rule_id: String },
    #[error("Failed to invoke report lambda")]
    LambdaInvocationFailed,
}

impl ErrorSwitch<ApiErrorResponse> for ReportError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            Self::InternalServerError => ApiErrorResponse::InternalServerError(ApiError::new(
                "RP",
                1,
                self.to_string(),
                None,
            )),
            Self::TimeRangeValidationError => {
                ApiErrorResponse::BadRequest(ApiError::new("RP", 2, self.to_string(), None))
            }
            Self::PayloadSerializationFailed => ApiErrorResponse::InternalServerError(
                ApiError::new("RP", 3, self.to_string(), None),
            ),
            Self::RuleNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("RP", 4, self.to_string(), None))
            }
            Self::LambdaInvocationFailed => ApiErrorResponse::InternalServerError(ApiError::new(
                "RP",
                5,
                self.to_string(),
                None,
            )),
        }
    }
}

pub type ReportResponse<T> = Result<ApplicationResponse<T>, Report<ReportError>>;
pub type ReportResult<T> = Result<T, Report<ReportError>>;
