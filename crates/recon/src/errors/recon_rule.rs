use error_stack::Report;
use thiserror::Error;

use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Clone, Error)]
pub enum ReconRuleError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Rule name cannot be empty")]
    InvalidRuleName,

    #[error("Rule description cannot be empty")]
    InvalidRuleDescription,

    #[error("At least one source must be provided")]
    NoSourcesProvided,

    #[error("At least one target must be provided")]
    NoTargetsProvided,

    #[error("Account not found: {account_id}")]
    AccountNotFound { account_id: String },

    #[error("Recon rule not found: {rule_id}")]
    RuleNotFound { rule_id: String },

    #[error("Invalid recon rule: {message}")]
    InvalidReconRule { message: String },

    #[error("Invalid search identifier: {message}")]
    InvalidSearchIdentifier { message: String },

    #[error("Invalid grouping field: {message}")]
    InvalidGroupingField { message: String },

    #[error("Invalid match rules: {message}")]
    InvalidMatchRules { message: String },
}

pub type ReconRuleResponse<T> = Result<ApplicationResponse<T>, Report<ReconRuleError>>;
pub type ReconRuleResult<T> = Result<T, Report<ReconRuleError>>;

impl From<ReconRuleError> for ReconAppError {
    fn from(error: ReconRuleError) -> Self {
        match error {
            ReconRuleError::InternalServerError => Self::InternalServerError,
            ReconRuleError::InvalidRuleName
            | ReconRuleError::InvalidRuleDescription
            | ReconRuleError::NoSourcesProvided
            | ReconRuleError::NoTargetsProvided
            | ReconRuleError::InvalidReconRule { .. }
            | ReconRuleError::InvalidSearchIdentifier { .. }
            | ReconRuleError::InvalidMatchRules { .. }
            | ReconRuleError::InvalidGroupingField { .. } => {
                Self::ValidationError(error.to_string())
            }
            ReconRuleError::AccountNotFound { .. } | ReconRuleError::RuleNotFound { .. } => {
                Self::ResourceNotFound(error.to_string())
            }
        }
    }
}

impl ErrorSwitchFrom<ReconRuleError> for ApiErrorPayload {
    fn switch_from(error: &ReconRuleError) -> Self {
        match error {
            ReconRuleError::InternalServerError => {
                Self::InternalServerError(ApiError::new("RR", 700, "Internal server error", None))
            }
            ReconRuleError::InvalidRuleName => {
                Self::BadRequest(ApiError::new("RR", 701, "Rule name cannot be empty", None))
            }
            ReconRuleError::InvalidRuleDescription => Self::BadRequest(ApiError::new(
                "RR",
                702,
                "Rule description cannot be empty",
                None,
            )),
            ReconRuleError::NoSourcesProvided => Self::BadRequest(ApiError::new(
                "RR",
                703,
                "At least one source must be provided",
                None,
            )),
            ReconRuleError::NoTargetsProvided => Self::BadRequest(ApiError::new(
                "RR",
                704,
                "At least one target must be provided",
                None,
            )),
            ReconRuleError::InvalidReconRule { message } => {
                Self::BadRequest(ApiError::new("RR", 705, message.clone(), None))
            }
            ReconRuleError::AccountNotFound { account_id } => Self::NotFound(ApiError::new(
                "RR",
                706,
                format!("Account not found: {account_id}"),
                None,
            )),
            ReconRuleError::RuleNotFound { rule_id } => Self::NotFound(ApiError::new(
                "RR",
                707,
                format!("Recon rule not found: {rule_id}"),
                None,
            )),
            ReconRuleError::InvalidSearchIdentifier { message } => {
                Self::BadRequest(ApiError::new("RR", 708, message.clone(), None))
            }
            ReconRuleError::InvalidMatchRules { message } => {
                Self::BadRequest(ApiError::new("RR", 709, message.clone(), None))
            }
            ReconRuleError::InvalidGroupingField { message } => {
                Self::BadRequest(ApiError::new("RR", 710, message.clone(), None))
            }
        }
    }
}
