use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum ReconEngineError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Staging entry not found: {id}")]
    StagingEntryNotFound { id: String },

    #[error("Staging entry already processed: {id}")]
    StagingEntryAlreadyProcessed { id: String },

    #[error("Staging entry voided: {id}")]
    StagingEntryVoided { id: String },

    #[error("Staging entry is archived: {id}")]
    StagingEntryArchived { id: String },

    #[error("Staging entry is invalid for duplicate processing: {id}")]
    InvalidForDuplicateProcessing { id: String },
}

pub type ReconEngineResult<T> = Result<T, Report<ReconEngineError>>;
pub type ReconEngineResponse<T> = Result<ApplicationResponse<T>, Report<ReconEngineError>>;

impl From<ReconEngineError> for ReconAppError {
    fn from(error: ReconEngineError) -> Self {
        match error {
            ReconEngineError::InternalServerError => Self::InternalServerError,
            ReconEngineError::StagingEntryNotFound { id } => {
                Self::ResourceNotFound(format!("Staging entry not found: {id}"))
            }
            ReconEngineError::StagingEntryAlreadyProcessed { id } => {
                Self::ValidationError(format!("Staging entry already processed: {id}"))
            }
            ReconEngineError::StagingEntryVoided { id } => {
                Self::ValidationError(format!("Staging entry voided: {id}"))
            }
            ReconEngineError::StagingEntryArchived { id } => {
                Self::ValidationError(format!("Staging entry is archived: {id}"))
            }
            ReconEngineError::InvalidForDuplicateProcessing { id } => Self::ValidationError(
                format!("Staging entry is invalid for duplicate processing: {id}"),
            ),
        }
    }
}

impl ErrorSwitchFrom<ReconEngineError> for ApiErrorPayload {
    fn switch_from(error: &ReconEngineError) -> Self {
        match error {
            ReconEngineError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                800,
                "An internal error occurred",
                None,
            )),
            ReconEngineError::StagingEntryNotFound { id } => Self::NotFound(ApiError::new(
                "RE",
                801,
                format!("Staging entry not found: {id}"),
                None,
            )),
            ReconEngineError::StagingEntryAlreadyProcessed { id } => {
                Self::BadRequest(ApiError::new(
                    "RE",
                    802,
                    format!("Staging entry already processed: {id}"),
                    None,
                ))
            }
            ReconEngineError::StagingEntryVoided { id } => Self::BadRequest(ApiError::new(
                "RE",
                803,
                format!("Staging entry voided: {id}"),
                None,
            )),
            ReconEngineError::StagingEntryArchived { id } => Self::BadRequest(ApiError::new(
                "RE",
                804,
                format!("Staging entry is archived: {id}"),
                None,
            )),
            ReconEngineError::InvalidForDuplicateProcessing { id } => {
                Self::BadRequest(ApiError::new(
                    "RE",
                    805,
                    format!("Staging entry is invalid for duplicate processing: {id}"),
                    None,
                ))
            }
        }
    }
}
