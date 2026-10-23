use common_enums::EntryLinkingStatus;
use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::types::ApplicationResponse;

#[derive(Debug, Clone, Error)]
pub enum LinkingProcessorError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Entry not found")]
    EntryNotFound,

    #[error(
        "Invalid entry linking status: {current_status}. Expected statuses: {expected_statuses:?}"
    )]
    InvalidEntryLinkingStatus {
        current_status: EntryLinkingStatus,
        expected_statuses: Vec<EntryLinkingStatus>,
    },
}

pub type LinkingProcessorResult<T> = Result<T, Report<LinkingProcessorError>>;
pub type LinkingProcessorResponse<T> =
    Result<ApplicationResponse<T>, Report<LinkingProcessorError>>;

impl ErrorSwitchFrom<LinkingProcessorError> for ApiErrorPayload {
    fn switch_from(error: &LinkingProcessorError) -> Self {
        match error {
            LinkingProcessorError::InternalServerError => Self::InternalServerError(ApiError::new(
                "LP",
                800,
                "An internal error occurred",
                None,
            )),
            LinkingProcessorError::EntryNotFound => {
                Self::NotFound(ApiError::new("LP", 801, "Entry not found", None))
            }
            LinkingProcessorError::InvalidEntryLinkingStatus { .. } => {
                Self::BadRequest(ApiError::new("LP", 802, error.to_string(), None))
            }
        }
    }
}
