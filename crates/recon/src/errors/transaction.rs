use common_enums::{Currency, DieselTransactionStatus, EntryStatus};
use error_stack::Report;
use thiserror::Error;

use api_models::{
    errors::{ApiError, ApiErrorResponse as ApiErrorPayload},
    transaction::TransactionListFilterValidationError,
};
use common_utils::errors::ErrorSwitchFrom;

use crate::{errors::ReconAppError, types::ApplicationResponse};

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("An internal error occurred")]
    InternalServerError,

    #[error("Transaction not found: {id}")]
    TransactionNotFound { id: String },

    #[error("Account not found: {account_id}")]
    AccountNotFound { account_id: String },

    #[error("Invalid amount: {details}")]
    InvalidAmount { details: String },

    #[error("Insufficient entries")]
    InsufficientEntries,

    #[error("Unbalanced transaction: {details}")]
    UnbalancedTransaction { details: String },

    #[error("Currency mismatch")]
    EntryCurrencyMismatch,

    #[error("Invalid rule ID: rule_id cannot be empty")]
    InvalidRuleId,

    #[error("Recon rule not found: {rule_id}")]
    ReconRuleNotFound { rule_id: String },

    #[error("Start time cannot be greater than End Time")]
    TimeRangeValidationError,

    #[error("Filtering by status '{status}' is not supported")]
    UnsupportedStatusFilter { status: String },

    #[error("Invalid transaction status. Expected statuses: {expected_statuses:?}")]
    InvalidTransactionStatus {
        expected_statuses: Vec<DieselTransactionStatus>,
    },

    #[error("Entry not found: {entry_id}")]
    EntryNotFound { entry_id: String },

    #[error("At least one staging entry ID is required")]
    EmptyStagingEntryList,

    #[error("Staging entry {staging_entry_id} cannot be linked to the transaction")]
    StagingEntryNotLinkable { staging_entry_id: String },

    #[error("Staging entry currency mismatch: staging entry {staging_entry_id} has currency {staging_currency:?} but transaction has currency {transaction_currency:?}")]
    StagingEntryCurrencyMismatch {
        staging_entry_id: String,
        staging_currency: Currency,
        transaction_currency: Currency,
    },

    #[error("Staging entry not found")]
    StagingEntryNotFound { staging_entry_id: String },

    #[error("Invalid entry status: {status:?}. Expected statuses: {expected_statuses:?}")]
    InvalidEntryStatus {
        status: EntryStatus,
        expected_statuses: Vec<EntryStatus>,
    },

    #[error("Transformation config not found: {config_id}")]
    TransformationConfigNotFound { config_id: String },
}

pub type TransactionResponse<T> = Result<ApplicationResponse<T>, Report<TransactionError>>;
pub type TransactionResult<T> = Result<T, Report<TransactionError>>;

impl From<TransactionError> for ReconAppError {
    fn from(error: TransactionError) -> Self {
        match error {
            TransactionError::InternalServerError => Self::InternalServerError,
            TransactionError::TransactionNotFound { .. }
            | TransactionError::AccountNotFound { .. }
            | TransactionError::ReconRuleNotFound { .. }
            | TransactionError::EntryNotFound { .. }
            | TransactionError::StagingEntryNotFound { .. }
            | TransactionError::TransformationConfigNotFound { .. } => {
                Self::ResourceNotFound(error.to_string())
            }
            TransactionError::InvalidAmount { .. }
            | TransactionError::InsufficientEntries
            | TransactionError::UnbalancedTransaction { .. }
            | TransactionError::EntryCurrencyMismatch
            | TransactionError::InvalidRuleId
            | TransactionError::TimeRangeValidationError
            | TransactionError::UnsupportedStatusFilter { .. }
            | TransactionError::InvalidTransactionStatus { .. }
            | TransactionError::EmptyStagingEntryList
            | TransactionError::StagingEntryNotLinkable { .. }
            | TransactionError::StagingEntryCurrencyMismatch { .. }
            | TransactionError::InvalidEntryStatus { .. } => {
                Self::ValidationError(error.to_string())
            }
        }
    }
}

impl ErrorSwitchFrom<TransactionError> for ApiErrorPayload {
    fn switch_from(error: &TransactionError) -> Self {
        match error {
            TransactionError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                600,
                "An internal error occurred while processing transaction request",
                None,
            )),
            TransactionError::TransactionNotFound { id } => Self::NotFound(ApiError::new(
                "RE",
                601,
                format!("Transaction not found: {id}"),
                None,
            )),
            TransactionError::AccountNotFound { account_id } => Self::NotFound(ApiError::new(
                "RE",
                602,
                format!("Account not found: {account_id}"),
                None,
            )),
            TransactionError::InvalidAmount { details } => Self::BadRequest(ApiError::new(
                "RE",
                603,
                format!("Invalid amount: {details}"),
                None,
            )),
            TransactionError::InsufficientEntries => Self::BadRequest(ApiError::new(
                "RE",
                604,
                "Insufficient entries".to_string(),
                None,
            )),
            TransactionError::UnbalancedTransaction { details } => Self::BadRequest(ApiError::new(
                "RE",
                606,
                format!("Unbalanced transaction: {details}"),
                None,
            )),
            TransactionError::EntryCurrencyMismatch => Self::BadRequest(ApiError::new(
                "RE",
                607,
                "Currency mismatch".to_string(),
                None,
            )),
            TransactionError::InvalidRuleId => Self::BadRequest(ApiError::new(
                "RE",
                608,
                "Invalid rule ID: rule_id cannot be empty".to_string(),
                None,
            )),
            TransactionError::TimeRangeValidationError => {
                Self::BadRequest(ApiError::new("RE", 609, error.to_string(), None))
            }
            TransactionError::InvalidTransactionStatus { expected_statuses } => {
                Self::BadRequest(ApiError::new(
                    "RE",
                    610,
                    format!("Invalid transaction status. Expected statuses: {expected_statuses:?}"),
                    None,
                ))
            }
            TransactionError::EntryNotFound { entry_id } => Self::NotFound(ApiError::new(
                "RE",
                611,
                format!("Entry not found: {entry_id}"),
                None,
            )),
            TransactionError::EmptyStagingEntryList => Self::BadRequest(ApiError::new(
                "RE",
                612,
                "At least one staging entry ID is required".to_string(),
                None,
            )),
            TransactionError::StagingEntryNotLinkable { staging_entry_id } => {
                Self::BadRequest(ApiError::new(
                    "RE",
                    613,
                    format!("Staging entry {staging_entry_id} cannot be linked due to its current state"),
                    None,
                ))
            }
            TransactionError::StagingEntryCurrencyMismatch {
                staging_entry_id,
                staging_currency,
                transaction_currency,
            } => Self::BadRequest(ApiError::new(
                "RE",
                614,
                format!(
                    "Staging entry currency mismatch: staging entry {staging_entry_id} has currency {staging_currency:?} but transaction has currency {transaction_currency:?}"
                ),
                None,
            )),
            TransactionError::StagingEntryNotFound { staging_entry_id } => {
                Self::NotFound(ApiError::new(
                    "RE",
                    615,
                    format!("Staging entry not found: {staging_entry_id}"),
                    None,
                ))
            }
            TransactionError::InvalidEntryStatus { status, expected_statuses } => Self::BadRequest(ApiError::new(
                "RE",
                616,
                format!("Invalid entry status: {status:?}. Expected statuses: {expected_statuses:?}"),
                None,
            )),
            TransactionError::ReconRuleNotFound { rule_id } => Self::NotFound(ApiError::new(
                "RE",
                617,
                format!("Recon rule not found: {rule_id}"),
                None,
            )),
            TransactionError::UnsupportedStatusFilter { status } => Self::BadRequest(ApiError::new(
                "RE",
                618,
                format!("Filtering by status '{status}' is not supported"),
                None,
            )),
            TransactionError::TransformationConfigNotFound { config_id } => Self::BadRequest(ApiError::new(
                "RE",
                619,
                format!("Transformation Config not found: {config_id}"),
                None,
            ))
        }
    }
}

impl ErrorSwitchFrom<TransactionListFilterValidationError> for TransactionError {
    fn switch_from(error: &TransactionListFilterValidationError) -> Self {
        match error {
            TransactionListFilterValidationError::InvalidTimeRange => {
                Self::TimeRangeValidationError
            }
            TransactionListFilterValidationError::UnsupportedStatus { status } => {
                Self::UnsupportedStatusFilter {
                    status: status.to_string(),
                }
            }
        }
    }
}
