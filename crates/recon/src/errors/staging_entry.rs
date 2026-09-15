use error_stack::Report;
use thiserror::Error;

use api_models::{
    errors::{ApiError, ApiErrorResponse as ApiErrorPayload},
    staging_entry::StagingEntryListFilterValidationError,
};
use common_utils::errors::ErrorSwitchFrom;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum StagingEntryError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Staging entry not found")]
    StagingEntryNotFound,

    #[error("Account not found: {account_id}")]
    AccountNotFound { account_id: String },

    #[error("Invalid amount: {details}")]
    InvalidAmount { details: String },

    #[error("Start time cannot be greater than End Time")]
    TimeRangeValidationError,

    #[error("Invalid status: {details}")]
    InvalidStatus { details: String },

    #[error("Transformation config not found: {transformation_id}")]
    TransformationConfigNotFound { transformation_id: String },

    #[error("Transformation config {transformation_id} does not belong to account {account_id}")]
    TransformationConfigAccountMismatch {
        transformation_id: String,
        account_id: String,
    },

    #[error("Metadata schema not found: {metadata_schema_id}")]
    MetadataSchemaNotFound { metadata_schema_id: String },

    #[error("Metadata schema validation failed: {details}")]
    MetadataSchemaValidationFailed { details: String },

    #[error("Staging entry {id} cannot be voided in its current state")]
    StagingEntryNotVoidable { id: String },

    #[error("Filtering by status '{status}' is not supported")]
    UnsupportedStatusFilter { status: String },
}

// Type alias for cleaner function signatures
pub type StagingEntryResponse<T> = Result<ApplicationResponse<T>, Report<StagingEntryError>>;
pub type StagingEntryResult<T> = Result<T, Report<StagingEntryError>>;

impl From<StagingEntryError> for ReconAppError {
    fn from(error: StagingEntryError) -> Self {
        match error {
            StagingEntryError::InternalServerError => Self::InternalServerError,
            StagingEntryError::StagingEntryNotFound => Self::ResourceNotFound(error.to_string()),
            StagingEntryError::AccountNotFound { .. } => Self::ResourceNotFound(error.to_string()),
            StagingEntryError::InvalidAmount { .. } => Self::ValidationError(error.to_string()),
            StagingEntryError::TimeRangeValidationError => Self::ValidationError(error.to_string()),
            StagingEntryError::InvalidStatus { .. } => Self::ValidationError(error.to_string()),
            StagingEntryError::TransformationConfigNotFound { .. } => {
                Self::ResourceNotFound(error.to_string())
            }
            StagingEntryError::TransformationConfigAccountMismatch { .. } => {
                Self::ValidationError(error.to_string())
            }
            StagingEntryError::MetadataSchemaNotFound { .. } => {
                Self::ResourceNotFound(error.to_string())
            }
            StagingEntryError::MetadataSchemaValidationFailed { .. } => {
                Self::ValidationError(error.to_string())
            }
            StagingEntryError::StagingEntryNotVoidable { .. } => {
                Self::ValidationError(error.to_string())
            }
            StagingEntryError::UnsupportedStatusFilter { .. } => {
                Self::ValidationError(error.to_string())
            }
        }
    }
}

impl ErrorSwitchFrom<StagingEntryError> for ApiErrorPayload {
    fn switch_from(error: &StagingEntryError) -> Self {
        match error {
            StagingEntryError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                400,
                "An internal error occurred while processing staging entry request",
                None,
            )),
            StagingEntryError::StagingEntryNotFound => {
                Self::NotFound(ApiError::new("RE", 401, "Staging entry not found", None))
            }
            StagingEntryError::InvalidAmount { details } => Self::BadRequest(ApiError::new(
                "RE",
                402,
                format!("Invalid amount format: {details}"),
                None,
            )),
            StagingEntryError::AccountNotFound { account_id } => Self::NotFound(ApiError::new(
                "RE",
                403,
                format!("Account not found: {account_id}"),
                None,
            )),
            StagingEntryError::TimeRangeValidationError => {
                Self::BadRequest(ApiError::new("RE", 404, error.to_string(), None))
            }
            StagingEntryError::InvalidStatus { details } => Self::BadRequest(ApiError::new(
                "RE",
                405,
                format!("Invalid status: {details}"),
                None,
            )),
            StagingEntryError::TransformationConfigNotFound { transformation_id } => {
                Self::NotFound(ApiError::new(
                    "RE",
                    406,
                    format!("Transformation config not found: {transformation_id}"),
                    None,
                ))
            }
            StagingEntryError::TransformationConfigAccountMismatch {
                transformation_id,
                account_id,
            } => Self::BadRequest(ApiError::new(
                "RE",
                407,
                format!(
                    "Transformation config {transformation_id} does not belong to account {account_id}"
                ),
                None,
            )),
            StagingEntryError::MetadataSchemaNotFound { metadata_schema_id } => {
                Self::NotFound(ApiError::new(
                    "RE",
                    408,
                    format!("Metadata schema not found: {metadata_schema_id}"),
                    None,
                ))
            }
            StagingEntryError::MetadataSchemaValidationFailed { details } => {
                Self::BadRequest(ApiError::new(
                    "RE",
                    409,
                    format!("Metadata schema validation failed: {details}"),
                    None,
                ))
            }
            StagingEntryError::StagingEntryNotVoidable { id } => {
                Self::BadRequest(ApiError::new(
                    "RE",
                    410,
                    format!("Staging entry {id} cannot be voided in its current state"),
                    None,
                ))
            }
            StagingEntryError::UnsupportedStatusFilter { status } => Self::BadRequest(
                ApiError::new(
                    "RE",
                    411,
                    format!("Filtering by status '{status}' is not supported"),
                    None,
                ),
            ),
        }
    }
}

impl ErrorSwitchFrom<StagingEntryListFilterValidationError> for StagingEntryError {
    fn switch_from(error: &StagingEntryListFilterValidationError) -> Self {
        match error {
            StagingEntryListFilterValidationError::InvalidTimeRange => {
                Self::TimeRangeValidationError
            }
            StagingEntryListFilterValidationError::UnsupportedStatus { status } => {
                Self::UnsupportedStatusFilter {
                    status: status.to_string(),
                }
            }
            StagingEntryListFilterValidationError::UnsupportedDetailedStatus { status } => {
                Self::UnsupportedStatusFilter {
                    status: status.to_string(),
                }
            }
        }
    }
}
