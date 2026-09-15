//! Resolution mapping logic for staging entries and transactions
//!
//! This module defines the ResolutionProvider trait and implements it for all
//! data types that can have exceptions requiring resolution actions.
//!
//! Part of the exception management domain, this module provides the foundation
//! for mapping exception types to available resolution actions.

use common_enums::ResolutionActionType;
use common_types::{
    staging_entry::{
        ManualReviewData, ProcessingErrorType, ProcessingErrorTypeV1, StagingEntryData,
        StagingEntryDataV2,
    },
    transaction::status::{
        DomainTransactionStatus, TransactionAmountMismatchData, TransactionMatchedData,
        TransactionPostedData,
    },
};
use recon_domain_models::transaction::DomainTransaction;

/// Trait for providing resolution actions for data states
pub trait ResolutionProvider {
    /// Get available resolution action types for this data state
    fn get_available_resolutions(&self) -> Vec<ResolutionActionType>;
}

// === Staging Entry Implementations ===

impl ResolutionProvider for StagingEntryData {
    fn get_available_resolutions(&self) -> Vec<ResolutionActionType> {
        match self {
            Self::V2(data_v2) => data_v2.get_available_resolutions(),
        }
    }
}

impl ResolutionProvider for StagingEntryDataV2 {
    fn get_available_resolutions(&self) -> Vec<ResolutionActionType> {
        match self {
            Self::Pending(_) => vec![],
            Self::Processed => vec![],
            Self::Void(_) => vec![],
            Self::NeedsManualReview(review_data) => review_data.get_available_resolutions(),
            Self::Archived => vec![],
        }
    }
}

impl ResolutionProvider for ManualReviewData {
    fn get_available_resolutions(&self) -> Vec<ResolutionActionType> {
        match self {
            Self::V1(v1_data) => v1_data.error_type.get_available_resolutions(),
        }
    }
}

impl ResolutionProvider for ProcessingErrorType {
    fn get_available_resolutions(&self) -> Vec<ResolutionActionType> {
        match self {
            Self::V1(error_v1) => error_v1.get_available_resolutions(),
        }
    }
}

impl ResolutionProvider for ProcessingErrorTypeV1 {
    fn get_available_resolutions(&self) -> Vec<ResolutionActionType> {
        match self {
            Self::NoRulesFound => vec![
                ResolutionActionType::CreateRule,
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::LinkStagingEntriesToTransaction,
                ResolutionActionType::VoidStagingEntry,
            ],

            Self::StagingEntryCurrencyMismatch { .. } => vec![
                ResolutionActionType::CreateAccount,
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::VoidStagingEntry,
            ],

            Self::DuplicateEntry { .. } => vec![
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::LinkStagingEntriesToTransaction,
                ResolutionActionType::ProcessDuplicateStagingEntry,
                ResolutionActionType::VoidStagingEntry,
            ],

            Self::NoExpectationEntryFound => vec![
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::LinkStagingEntriesToTransaction,
                ResolutionActionType::VoidStagingEntry,
            ],

            Self::MultipleExpectedEntriesFound { .. } => vec![
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::LinkStagingEntriesToTransaction,
                ResolutionActionType::VoidStagingEntry,
            ],

            Self::MissingSearchIdentifierValue { .. } | Self::MissingMatchField { .. } => vec![
                ResolutionActionType::CreateRule,
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::LinkStagingEntriesToTransaction,
                ResolutionActionType::VoidStagingEntry,
            ],

            Self::MissingUniqueField { .. } => vec![
                ResolutionActionType::CreateRule,
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::VoidStagingEntry,
            ],

            Self::MissingGroupingField { .. } => vec![
                ResolutionActionType::CreateRule,
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::VoidStagingEntry,
            ],

            Self::InternalError => vec![
                ResolutionActionType::EditStagingEntry,
                ResolutionActionType::VoidStagingEntry,
            ],
        }
    }
}

// === Transaction Implementations ===

impl ResolutionProvider for DomainTransaction {
    fn get_available_resolutions(&self) -> Vec<ResolutionActionType> {
        match self.status {
            DomainTransactionStatus::Expected
            | DomainTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch)
            | DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected)
            | DomainTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected)
            | DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch) => {
                vec![
                    ResolutionActionType::CreateEntries,
                    ResolutionActionType::LinkStagingEntriesToTransaction,
                    ResolutionActionType::ReplaceEntries,
                    ResolutionActionType::VoidTransaction,
                ]
            }
            DomainTransactionStatus::Posted(TransactionPostedData::Manual) => {
                vec![]
            }
            DomainTransactionStatus::Matched(TransactionMatchedData::Auto)
            | DomainTransactionStatus::Matched(TransactionMatchedData::Manual)
            | DomainTransactionStatus::Matched(TransactionMatchedData::Force)
            | DomainTransactionStatus::Matched(TransactionMatchedData::WithTolerance) => {
                vec![ResolutionActionType::VoidTransaction]
            }
            DomainTransactionStatus::SplitMismatch | DomainTransactionStatus::DataMismatch => {
                vec![
                    ResolutionActionType::CreateEntries,
                    ResolutionActionType::ForceReconcile,
                    ResolutionActionType::LinkStagingEntriesToTransaction,
                    ResolutionActionType::ReplaceEntries,
                    ResolutionActionType::VoidTransaction,
                ]
            }
            DomainTransactionStatus::CurrencyMismatch => vec![
                ResolutionActionType::ReplaceEntries,
                ResolutionActionType::VoidTransaction,
            ],
            DomainTransactionStatus::Archived => vec![],
            DomainTransactionStatus::Void => vec![],
            DomainTransactionStatus::PartiallyReconciled => vec![
                ResolutionActionType::CreateEntries,
                ResolutionActionType::LinkStagingEntriesToTransaction,
                ResolutionActionType::ReplaceEntries,
                ResolutionActionType::VoidTransaction,
            ],
        }
    }
}
