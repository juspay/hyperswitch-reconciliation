use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;
use error_stack::Report;
use thiserror::Error;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum LinkConfigError {
    #[error("An internal error occurred")]
    InternalServerError,
    #[error("Link config not found: {link_config_id}")]
    LinkConfigNotFound { link_config_id: String },
    #[error("Account not found: {account_id}")]
    AccountNotFound { account_id: String },
    #[error("Recon rule not found: {rule_id}")]
    RuleNotFound { rule_id: String },
    #[error("Invalid link config: {message}")]
    InvalidLinkConfig { message: String },
}

pub type LinkConfigResult<T> = Result<T, Report<LinkConfigError>>;
pub type LinkConfigResponse<T> = Result<ApplicationResponse<T>, Report<LinkConfigError>>;

impl From<LinkConfigError> for ReconAppError {
    fn from(error: LinkConfigError) -> Self {
        match error {
            LinkConfigError::InternalServerError => Self::InternalServerError,
            LinkConfigError::InvalidLinkConfig { .. } => Self::ValidationError(error.to_string()),
            LinkConfigError::LinkConfigNotFound { .. }
            | LinkConfigError::AccountNotFound { .. }
            | LinkConfigError::RuleNotFound { .. } => Self::ResourceNotFound(error.to_string()),
        }
    }
}

impl ErrorSwitchFrom<LinkConfigError> for ApiErrorPayload {
    fn switch_from(error: &LinkConfigError) -> Self {
        match error {
            LinkConfigError::InternalServerError => {
                Self::InternalServerError(ApiError::new("LC", 900, "Internal server error", None))
            }
            LinkConfigError::LinkConfigNotFound { .. } => {
                Self::NotFound(ApiError::new("LC", 901, error.to_string(), None))
            }
            LinkConfigError::AccountNotFound { .. } => {
                Self::NotFound(ApiError::new("LC", 902, error.to_string(), None))
            }
            LinkConfigError::RuleNotFound { .. } => {
                Self::NotFound(ApiError::new("LC", 903, error.to_string(), None))
            }
            LinkConfigError::InvalidLinkConfig { .. } => {
                Self::BadRequest(ApiError::new("LC", 904, error.to_string(), None))
            }
        }
    }
}
