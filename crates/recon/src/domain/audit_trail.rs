use api_models::{audit_trail::AuditEvent, AccountData};
use error_stack::ResultExt;
use std::collections::HashMap;

use crate::errors::audit_trail::AuditTrailError;

/// Time window for grouping events (in minutes)
pub const TIME_WINDOW_MINUTES: i64 = 5;

/// Grouping strategy for event occurrences
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupingStrategy {
    /// No grouping - each occurrence is a separate event
    None,
    /// Consecutive grouping only - merge only if they appear consecutively
    Consecutive,
    /// Time window grouping - merge if within time window
    TimeWindow,
}

impl GroupingStrategy {
    /// Determines if a new occurrence should be merged with an existing group
    /// based on the grouping strategy, account ID(s), and timestamps
    pub fn should_merge<T: Eq>(
        &self,
        existing_account_ids: &T,
        new_account_ids: &T,
        existing_timestamp: &time::PrimitiveDateTime,
        new_timestamp: &time::PrimitiveDateTime,
    ) -> bool {
        match self {
            Self::None => false,
            Self::Consecutive => existing_account_ids == new_account_ids,
            Self::TimeWindow => {
                existing_account_ids == new_account_ids
                    && (*new_timestamp - *existing_timestamp).whole_minutes().abs()
                        <= TIME_WINDOW_MINUTES
            }
        }
    }
}

/// Individual occurrence structs (data only, no timestamp)
#[derive(Debug, Clone)]
pub struct FileUploadedOccurrence {
    pub account_id: String,
    pub ingestion_id: String,
    pub ingestion_history_id: String,
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub struct IngestionFailedOccurrence {
    pub account_id: String,
}

#[derive(Debug, Clone)]
pub struct StagingEntryCreatedOccurrence {
    pub account_id: String,
}

#[derive(Debug, Clone)]
pub struct StagingEntryNeedsManualReviewOccurrence {
    pub account_id: String,
}

#[derive(Debug, Clone)]
pub struct ExpectationCreatedOccurrence {
    pub account_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionReconciledOccurrence {
    pub account_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionMatchedOccurrence {
    pub account_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionMismatchedOccurrence {
    pub account_ids: Vec<String>,
}

/// Individual audit event occurrence (ungrouped)
/// Timestamp is at enum level as metadata
#[derive(Debug, Clone)]
pub enum AuditEventOccurrence {
    FileUploaded {
        data: FileUploadedOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
    IngestionFailed {
        data: IngestionFailedOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
    StagingEntryCreated {
        data: StagingEntryCreatedOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
    StagingEntryNeedsManualReview {
        data: StagingEntryNeedsManualReviewOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
    ExpectationCreated {
        data: ExpectationCreatedOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
    TransactionMatched {
        data: TransactionMatchedOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
    TransactionReconciled {
        data: TransactionReconciledOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
    TransactionMismatched {
        data: TransactionMismatchedOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
}

impl AuditEventOccurrence {
    pub fn timestamp(&self) -> &time::PrimitiveDateTime {
        match self {
            Self::FileUploaded { timestamp, .. } => timestamp,
            Self::IngestionFailed { timestamp, .. } => timestamp,
            Self::StagingEntryCreated { timestamp, .. } => timestamp,
            Self::StagingEntryNeedsManualReview { timestamp, .. } => timestamp,
            Self::ExpectationCreated { timestamp, .. } => timestamp,
            Self::TransactionMatched { timestamp, .. } => timestamp,
            Self::TransactionReconciled { timestamp, .. } => timestamp,
            Self::TransactionMismatched { timestamp, .. } => timestamp,
        }
    }

    pub fn account_ids(&self) -> Vec<String> {
        match self {
            Self::FileUploaded { data, .. } => vec![data.account_id.clone()],
            Self::IngestionFailed { data, .. } => vec![data.account_id.clone()],
            Self::StagingEntryCreated { data, .. } => vec![data.account_id.clone()],
            Self::StagingEntryNeedsManualReview { data, .. } => vec![data.account_id.clone()],
            Self::ExpectationCreated { data, .. } => data.account_ids.clone(),
            Self::TransactionMatched { data, .. } => data.account_ids.clone(),
            Self::TransactionReconciled { data, .. } => data.account_ids.clone(),
            Self::TransactionMismatched { data, .. } => data.account_ids.clone(),
        }
    }

    /// Returns the grouping strategy for this event type
    pub fn grouping_strategy(&self) -> GroupingStrategy {
        match self {
            Self::FileUploaded { .. } => GroupingStrategy::None,
            Self::IngestionFailed { .. } => GroupingStrategy::Consecutive,
            Self::StagingEntryCreated { .. }
            | Self::StagingEntryNeedsManualReview { .. }
            | Self::ExpectationCreated { .. }
            | Self::TransactionMatched { .. }
            | Self::TransactionReconciled { .. }
            | Self::TransactionMismatched { .. } => GroupingStrategy::TimeWindow,
        }
    }
}

/// Grouped audit events - type-safe container for grouped occurrences
/// FileUploaded is not grouped (single occurrence), others are Vec for grouping
#[derive(Debug, Clone)]
pub enum GroupedAuditEvent {
    FileUploaded {
        data: FileUploadedOccurrence,
        timestamp: time::PrimitiveDateTime,
    },
    IngestionsFailed {
        data: Vec<IngestionFailedOccurrence>,
        timestamp: time::PrimitiveDateTime,
        account_id: String,
    },
    StagingEntriesCreated {
        data: Vec<StagingEntryCreatedOccurrence>,
        timestamp: time::PrimitiveDateTime,
        account_id: String,
    },
    StagingEntryNeedsManualReview {
        data: Vec<StagingEntryNeedsManualReviewOccurrence>,
        timestamp: time::PrimitiveDateTime,
        account_id: String,
    },
    ExpectationsCreated {
        data: Vec<ExpectationCreatedOccurrence>,
        timestamp: time::PrimitiveDateTime,
        account_ids: Vec<String>,
    },
    TransactionsMatched {
        data: Vec<TransactionMatchedOccurrence>,
        timestamp: time::PrimitiveDateTime,
        account_ids: Vec<String>,
    },
    TransactionsReconciled {
        data: Vec<TransactionReconciledOccurrence>,
        timestamp: time::PrimitiveDateTime,
        account_ids: Vec<String>,
    },
    TransactionsMismatched {
        data: Vec<TransactionMismatchedOccurrence>,
        timestamp: time::PrimitiveDateTime,
        account_ids: Vec<String>,
    },
}

impl GroupedAuditEvent {
    /// Get the timestamp for sorting
    pub fn timestamp(&self) -> time::PrimitiveDateTime {
        match self {
            Self::FileUploaded { timestamp, .. } => *timestamp,
            Self::IngestionsFailed { timestamp, .. } => *timestamp,
            Self::StagingEntriesCreated { timestamp, .. } => *timestamp,
            Self::StagingEntryNeedsManualReview { timestamp, .. } => *timestamp,
            Self::ExpectationsCreated { timestamp, .. } => *timestamp,
            Self::TransactionsMatched { timestamp, .. } => *timestamp,
            Self::TransactionsReconciled { timestamp, .. } => *timestamp,
            Self::TransactionsMismatched { timestamp, .. } => *timestamp,
        }
    }

    pub fn merge_occurrence(&mut self, occurrence: &AuditEventOccurrence) -> bool {
        match self {
            Self::FileUploaded { .. } => return false, // FileUploaded doesn't group
            Self::IngestionsFailed {
                data,
                timestamp,
                account_id,
            } => {
                if let AuditEventOccurrence::IngestionFailed {
                    data: occ_data,
                    timestamp: occ_timestamp,
                } = occurrence
                {
                    if occurrence.grouping_strategy().should_merge(
                        account_id,
                        &occ_data.account_id,
                        timestamp,
                        occ_timestamp,
                    ) {
                        data.push(occ_data.clone());
                        *timestamp = *occ_timestamp;
                        return true;
                    }
                }
            }
            Self::StagingEntriesCreated {
                data,
                timestamp,
                account_id,
            } => {
                if let AuditEventOccurrence::StagingEntryCreated {
                    data: occ_data,
                    timestamp: occ_timestamp,
                } = occurrence
                {
                    if occurrence.grouping_strategy().should_merge(
                        account_id,
                        &occ_data.account_id,
                        timestamp,
                        occ_timestamp,
                    ) {
                        data.push(occ_data.clone());
                        *timestamp = *occ_timestamp;
                        return true;
                    }
                }
            }
            Self::StagingEntryNeedsManualReview {
                data,
                timestamp,
                account_id,
            } => {
                if let AuditEventOccurrence::StagingEntryNeedsManualReview {
                    data: occ_data,
                    timestamp: occ_timestamp,
                } = occurrence
                {
                    if occurrence.grouping_strategy().should_merge(
                        account_id,
                        &occ_data.account_id,
                        timestamp,
                        occ_timestamp,
                    ) {
                        data.push(occ_data.clone());
                        *timestamp = *occ_timestamp;
                        return true;
                    }
                }
            }
            Self::ExpectationsCreated {
                data,
                timestamp,
                account_ids,
            } => {
                if let AuditEventOccurrence::ExpectationCreated {
                    data: occ_data,
                    timestamp: occ_timestamp,
                } = occurrence
                {
                    if occurrence.grouping_strategy().should_merge(
                        account_ids,
                        &occ_data.account_ids,
                        timestamp,
                        occ_timestamp,
                    ) {
                        data.push(occ_data.clone());
                        *timestamp = *occ_timestamp;
                        return true;
                    }
                }
            }
            Self::TransactionsMatched {
                data,
                timestamp,
                account_ids,
            } => {
                if let AuditEventOccurrence::TransactionMatched {
                    data: occ_data,
                    timestamp: occ_timestamp,
                } = occurrence
                {
                    if occurrence.grouping_strategy().should_merge(
                        account_ids,
                        &occ_data.account_ids,
                        timestamp,
                        occ_timestamp,
                    ) {
                        data.push(occ_data.clone());
                        *timestamp = *occ_timestamp;
                        return true;
                    }
                }
            }
            Self::TransactionsReconciled {
                data,
                timestamp,
                account_ids,
            } => {
                if let AuditEventOccurrence::TransactionReconciled {
                    data: occ_data,
                    timestamp: occ_timestamp,
                } = occurrence
                {
                    if occurrence.grouping_strategy().should_merge(
                        account_ids,
                        &occ_data.account_ids,
                        timestamp,
                        occ_timestamp,
                    ) {
                        data.push(occ_data.clone());
                        *timestamp = *occ_timestamp;
                        return true;
                    }
                }
            }
            Self::TransactionsMismatched {
                data,
                timestamp,
                account_ids,
            } => {
                if let AuditEventOccurrence::TransactionMismatched {
                    data: occ_data,
                    timestamp: occ_timestamp,
                } = occurrence
                {
                    if occurrence.grouping_strategy().should_merge(
                        account_ids,
                        &occ_data.account_ids,
                        timestamp,
                        occ_timestamp,
                    ) {
                        data.push(occ_data.clone());
                        *timestamp = *occ_timestamp;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Convert a GroupedAuditEvent to an AuditEvent with account names
    pub fn to_api_response(
        self,
        account_map: &HashMap<String, AccountData>,
    ) -> Result<AuditEvent, error_stack::Report<AuditTrailError>> {
        match self {
            Self::FileUploaded { data, timestamp } => {
                let account = account_map
                    .get(&data.account_id)
                    .ok_or(AuditTrailError::InternalServerError)
                    .attach_printable_lazy(|| format!("Account {} not found", data.account_id))?
                    .clone();

                Ok(AuditEvent::FileUploaded {
                    account,
                    ingestion_id: data.ingestion_id,
                    ingestion_history_id: data.ingestion_history_id,
                    file_name: data.file_name,
                    timestamp,
                })
            }
            Self::IngestionsFailed {
                data,
                timestamp,
                account_id,
            } => {
                let account = account_map
                    .get(&account_id)
                    .ok_or(AuditTrailError::InternalServerError)
                    .attach_printable_lazy(|| format!("Account {account_id} not found"))?
                    .clone();

                Ok(AuditEvent::IngestionsFailed {
                    account,
                    count: data.len(),
                    last_failed_at: timestamp,
                })
            }
            Self::StagingEntriesCreated {
                data,
                timestamp,
                account_id,
            } => {
                let account = account_map
                    .get(&account_id)
                    .ok_or(AuditTrailError::InternalServerError)
                    .attach_printable_lazy(|| format!("Account {account_id} not found"))?
                    .clone();

                Ok(AuditEvent::StagingEntriesCreated {
                    account,
                    count: data.len(),
                    timestamp,
                })
            }
            Self::StagingEntryNeedsManualReview {
                data,
                timestamp,
                account_id,
            } => {
                let account = account_map
                    .get(&account_id)
                    .ok_or(AuditTrailError::InternalServerError)
                    .attach_printable_lazy(|| format!("Account {account_id} not found"))?
                    .clone();

                Ok(AuditEvent::StagingEntryNeedsManualReview {
                    account,
                    count: data.len(),
                    timestamp,
                })
            }
            Self::ExpectationsCreated {
                data,
                timestamp,
                account_ids,
            } => {
                let accounts = account_ids
                    .iter()
                    .map(|account_id| {
                        account_map
                            .get(account_id)
                            .ok_or(AuditTrailError::InternalServerError)
                            .attach_printable_lazy(|| format!("Account {account_id} not found"))
                            .cloned()
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(AuditEvent::ExpectationsCreated {
                    accounts,
                    count: data.len(),
                    timestamp,
                })
            }
            Self::TransactionsMatched {
                data,
                timestamp,
                account_ids,
            } => {
                let accounts = account_ids
                    .iter()
                    .map(|account_id| {
                        account_map
                            .get(account_id)
                            .ok_or(AuditTrailError::InternalServerError)
                            .attach_printable_lazy(|| format!("Account {account_id} not found"))
                            .cloned()
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(AuditEvent::TransactionsMatched {
                    accounts,
                    count: data.len(),
                    timestamp,
                })
            }
            Self::TransactionsReconciled {
                data,
                timestamp,
                account_ids,
            } => {
                let accounts = account_ids
                    .iter()
                    .map(|account_id| {
                        account_map
                            .get(account_id)
                            .ok_or(AuditTrailError::InternalServerError)
                            .attach_printable_lazy(|| format!("Account {account_id} not found"))
                            .cloned()
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(AuditEvent::TransactionsReconciled {
                    accounts,
                    count: data.len(),
                    timestamp,
                })
            }
            Self::TransactionsMismatched {
                data,
                timestamp,
                account_ids,
            } => {
                let accounts = account_ids
                    .iter()
                    .map(|account_id| {
                        account_map
                            .get(account_id)
                            .ok_or(AuditTrailError::InternalServerError)
                            .attach_printable_lazy(|| format!("Account {account_id} not found"))
                            .cloned()
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(AuditEvent::TransactionsMismatched {
                    accounts,
                    count: data.len(),
                    timestamp,
                })
            }
        }
    }
}
