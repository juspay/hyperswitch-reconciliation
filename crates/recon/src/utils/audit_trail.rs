use api_models::{audit_trail::AuditEvent, staging_entry::ApiStagingEntryStatus};
use common_enums::{DieselStagingEntryStatus, DieselTransactionStatus, IngestionStatus};
use common_types::{
    ingestion::ListIngestionHistoryFilters,
    staging_entry::{status::DomainStagingEntryStatus, ListStagingEntriesFilters},
    transaction::{
        status::{
            DomainTransactionStatus, TransactionAmountMismatchData, TransactionMatchedData,
            TransactionPostedData,
        },
        TransactionAccountFilters,
    },
};
use common_utils::auth::AuthContext;
use error_stack::ResultExt;
use std::ops::Not;

use crate::{
    db::{ingestion_history::IngestionHistoryInterface, transaction::TransactionInterface},
    domain::audit_trail::{
        AuditEventOccurrence, ExpectationCreatedOccurrence, FileUploadedOccurrence,
        GroupedAuditEvent, GroupingStrategy, IngestionFailedOccurrence,
        StagingEntryCreatedOccurrence, StagingEntryNeedsManualReviewOccurrence,
        TransactionMatchedOccurrence, TransactionMismatchedOccurrence,
        TransactionReconciledOccurrence,
    },
    errors::audit_trail::AuditTrailError,
    routes::AppState,
    utils::{
        ingestion::IngestionDataExt, staging_entry::ApiStagingEntryStatusExt,
        transaction::group_transaction_views,
    },
};

pub async fn get_ingestion_events(
    state: &AppState,
    profile_id: &str,
    time_range: &Option<common_types::date_time::TimeRange>,
) -> Result<Vec<AuditEventOccurrence>, error_stack::Report<AuditTrailError>> {
    // Query database with filters - only get Processed and Failed statuses to exclude Discarded
    let ingestion_history = state
        .store
        .list_history_with_config_by_profile_id(
            profile_id,
            ListIngestionHistoryFilters {
                ingestion_id: None,
                ingestion_history_id: None,
                account_id: None,
                status: Some(vec![IngestionStatus::Processed, IngestionStatus::Failed]),
                time_range: *time_range,
            },
        )
        .await
        .change_context(AuditTrailError::InternalServerError)?;

    // Create individual event occurrences (not grouped)
    ingestion_history
        .iter()
        .map(|view| match view.history.status {
            IngestionStatus::Processed => {
                let file_name = view
                    .history
                    .data
                    .as_ref()
                    .and_then(|data| data.get_file_name())
                    .ok_or(AuditTrailError::InternalServerError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Missing file name for processed ingestion_history_id {}",
                            view.history.ingestion_history_id
                        )
                    })?;

                Ok(AuditEventOccurrence::FileUploaded {
                    data: FileUploadedOccurrence {
                        account_id: view.history.account_id.clone(),
                        ingestion_id: view.history.ingestion_config_id.clone(),
                        ingestion_history_id: view.history.ingestion_history_id.clone(),
                        file_name,
                    },
                    timestamp: view.history.created_at,
                })
            }
            IngestionStatus::Failed => Ok(AuditEventOccurrence::IngestionFailed {
                data: IngestionFailedOccurrence {
                    account_id: view.history.account_id.clone(),
                },
                timestamp: view.history.created_at,
            }),
            IngestionStatus::Pending | IngestionStatus::Processing | IngestionStatus::Discarded => {
                Err(
                    error_stack::Report::new(AuditTrailError::InternalServerError)
                        .attach_printable("Unexpected ingestion status"),
                )
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

pub async fn get_staging_entry_events(
    state: &AppState,
    profile_id: &str,
    time_range: &Option<common_types::date_time::TimeRange>,
) -> Result<Vec<AuditEventOccurrence>, error_stack::Report<AuditTrailError>> {
    // Query database with filters - exclude Archived entries
    let staging_entries = state
        .store
        .list_staging_entries_by_profile_id(
            profile_id,
            ListStagingEntriesFilters {
                account_id: None,
                status: Some(
                    [
                        ApiStagingEntryStatus::NeedsManualReview(None),
                        ApiStagingEntryStatus::Pending,
                        ApiStagingEntryStatus::Processed,
                    ]
                    .into_iter()
                    .flat_map(ApiStagingEntryStatus::to_domain_statuses)
                    .map(DieselStagingEntryStatus::from)
                    .collect(),
                ),
                time_range: *time_range,
                transformation_history_id: None,
                entry_type: None,
                currency: None,
                staging_entry_id: None,
            },
            None,
        )
        .await
        .change_context(AuditTrailError::InternalServerError)?;

    // Create individual event occurrences (not grouped)
    staging_entries
        .iter()
        .filter(|view| view.staging_entry.discarded_at.is_none()) // Exclude discarded
        .map(
            |view| match DomainStagingEntryStatus::from(view.staging_entry.status) {
                DomainStagingEntryStatus::NeedsManualReview(_) => {
                    Ok(AuditEventOccurrence::StagingEntryNeedsManualReview {
                        data: StagingEntryNeedsManualReviewOccurrence {
                            account_id: view.account.id.clone(),
                        },
                        timestamp: view.staging_entry.created_at,
                    })
                }
                DomainStagingEntryStatus::Pending | DomainStagingEntryStatus::Processed => {
                    Ok(AuditEventOccurrence::StagingEntryCreated {
                        data: StagingEntryCreatedOccurrence {
                            account_id: view.account.id.clone(),
                        },
                        timestamp: view.staging_entry.created_at,
                    })
                }
                DomainStagingEntryStatus::Archived | DomainStagingEntryStatus::Void => Err(
                    error_stack::Report::new(AuditTrailError::InternalServerError)
                        .attach_printable("Archived staging entry should have been filtered"),
                ),
            },
        )
        .collect::<Result<Vec<_>, _>>()
}

pub async fn get_transaction_events(
    state: &AppState,
    auth_context: &AuthContext,
    time_range: &Option<common_types::date_time::TimeRange>,
) -> Result<Vec<AuditEventOccurrence>, error_stack::Report<AuditTrailError>> {
    let encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    let transaction_account_filters = TransactionAccountFilters {
        transaction_id: None,
        status: Some(vec![
            DieselTransactionStatus::Expected,
            DieselTransactionStatus::PostedManual,
            DieselTransactionStatus::MatchedAuto,
            DieselTransactionStatus::MatchedManual,
            DieselTransactionStatus::MatchedForce,
            DieselTransactionStatus::OverAmountExpected,
            DieselTransactionStatus::OverAmountMismatch,
            DieselTransactionStatus::UnderAmountExpected,
            DieselTransactionStatus::UnderAmountMismatch,
            DieselTransactionStatus::DataMismatch,
        ]),
        time_range: *time_range,
        rule_id: None,
        credit_account: None,
        debit_account: None,
    };

    // Query database with filters
    let transactions = state
        .store
        .list_with_accounts_and_entries(&auth_context.profile_id, transaction_account_filters, None)
        .await
        .change_context(AuditTrailError::InternalServerError)?;

    let transaction_groups = group_transaction_views(&encryptor, transactions)
        .await
        .change_context(AuditTrailError::InternalServerError)?;

    // Create individual event occurrences (not grouped)
    transaction_groups
        .into_iter()
        .filter(|group| group.transaction.discarded_at.is_none()) // Exclude discarded
        .map(|group| {
            // Get the transaction
            let txn = group.transaction;

            // Collect unique account_ids from entries_with_accounts and sort for consistent ordering
            let mut account_ids: Vec<String> = group
                .entries_with_accounts
                .into_iter()
                .map(|(_, account)| account.id)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            account_ids.sort();

            match txn.status {
                DomainTransactionStatus::Expected => Ok(AuditEventOccurrence::ExpectationCreated {
                    data: ExpectationCreatedOccurrence { account_ids },
                    timestamp: txn.created_at,
                }),
                DomainTransactionStatus::Matched(TransactionMatchedData::Auto)
                | DomainTransactionStatus::Matched(TransactionMatchedData::Manual)
                | DomainTransactionStatus::Matched(TransactionMatchedData::Force)
                | DomainTransactionStatus::Matched(TransactionMatchedData::WithTolerance) => {
                    Ok(AuditEventOccurrence::TransactionMatched {
                        data: TransactionMatchedOccurrence { account_ids },
                        timestamp: txn.created_at,
                    })
                }
                DomainTransactionStatus::Posted(TransactionPostedData::Manual) => {
                    Ok(AuditEventOccurrence::TransactionReconciled {
                        data: TransactionReconciledOccurrence { account_ids },
                        timestamp: txn.created_at,
                    })
                }
                DomainTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch)
                | DomainTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected)
                | DomainTransactionStatus::DataMismatch
                | DomainTransactionStatus::CurrencyMismatch
                | DomainTransactionStatus::SplitMismatch
                | DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch)
                | DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected) => {
                    Ok(AuditEventOccurrence::TransactionMismatched {
                        data: TransactionMismatchedOccurrence { account_ids },
                        timestamp: txn.created_at,
                    })
                }
                DomainTransactionStatus::Archived
                | DomainTransactionStatus::Void
                | DomainTransactionStatus::PartiallyReconciled => Err(error_stack::Report::new(
                    AuditTrailError::InternalServerError,
                )
                .attach_printable("Unexpected transaction status")),
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

/// Group occurrences into GroupedAuditEvents based on their grouping strategy
pub fn group_occurrences_into_grouped_events(
    occurrences: Vec<AuditEventOccurrence>,
) -> Vec<GroupedAuditEvent> {
    occurrences.into_iter().fold(
        Vec::new(),
        |mut grouped_events: Vec<GroupedAuditEvent>, occurrence| {
            match occurrence.grouping_strategy() {
                GroupingStrategy::None => {
                    // No grouping, create new grouped event
                    grouped_events.push(occurrence.into());
                }
                GroupingStrategy::Consecutive => {
                    // Check last grouped event for possible merge
                    grouped_events
                        .last_mut()
                        .is_some_and(|last| last.merge_occurrence(&occurrence))
                        .not()
                        .then(|| {
                            grouped_events.push(occurrence.into());
                        });
                }
                GroupingStrategy::TimeWindow => {
                    // Check existing grouped events in reverse for possible merge
                    grouped_events
                        .iter_mut()
                        .rev()
                        .any(|group| group.merge_occurrence(&occurrence))
                        .not()
                        .then(|| {
                            grouped_events.push(occurrence.into());
                        });
                }
            }
            grouped_events
        },
    )
}

impl From<AuditEventOccurrence> for GroupedAuditEvent {
    fn from(occurrence: AuditEventOccurrence) -> Self {
        match occurrence {
            AuditEventOccurrence::FileUploaded { data, timestamp } => {
                Self::FileUploaded { data, timestamp }
            }
            AuditEventOccurrence::IngestionFailed { data, timestamp } => Self::IngestionsFailed {
                account_id: data.account_id.clone(),
                data: vec![data],
                timestamp,
            },
            AuditEventOccurrence::StagingEntryCreated { data, timestamp } => {
                Self::StagingEntriesCreated {
                    account_id: data.account_id.clone(),
                    data: vec![data],
                    timestamp,
                }
            }
            AuditEventOccurrence::StagingEntryNeedsManualReview { data, timestamp } => {
                Self::StagingEntryNeedsManualReview {
                    account_id: data.account_id.clone(),
                    data: vec![data],
                    timestamp,
                }
            }
            AuditEventOccurrence::ExpectationCreated { data, timestamp } => {
                Self::ExpectationsCreated {
                    account_ids: data.account_ids.clone(),
                    data: vec![data],
                    timestamp,
                }
            }
            AuditEventOccurrence::TransactionMatched { data, timestamp } => {
                Self::TransactionsMatched {
                    account_ids: data.account_ids.clone(),
                    data: vec![data],
                    timestamp,
                }
            }
            AuditEventOccurrence::TransactionReconciled { data, timestamp } => {
                Self::TransactionsReconciled {
                    account_ids: data.account_ids.clone(),
                    data: vec![data],
                    timestamp,
                }
            }
            AuditEventOccurrence::TransactionMismatched { data, timestamp } => {
                Self::TransactionsMismatched {
                    account_ids: data.account_ids.clone(),
                    data: vec![data],
                    timestamp,
                }
            }
        }
    }
}

pub trait AuditEventExt {
    fn timestamp(&self) -> &time::PrimitiveDateTime;
}

impl AuditEventExt for AuditEvent {
    fn timestamp(&self) -> &time::PrimitiveDateTime {
        match self {
            Self::FileUploaded { timestamp, .. } => timestamp,
            Self::IngestionsFailed { last_failed_at, .. } => last_failed_at,
            Self::StagingEntriesCreated { timestamp, .. } => timestamp,
            Self::StagingEntryNeedsManualReview { timestamp, .. } => timestamp,
            Self::ExpectationsCreated { timestamp, .. } => timestamp,
            Self::TransactionsMatched { timestamp, .. } => timestamp,
            Self::TransactionsReconciled { timestamp, .. } => timestamp,
            Self::TransactionsMismatched { timestamp, .. } => timestamp,
        }
    }
}
