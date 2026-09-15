use common_types::date_time::TimeRange;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::AccountData;

/// Query parameters for audit trail endpoint
#[derive(Debug, Clone, Default, Deserialize, IntoParams, ToSchema)]
pub struct AuditTrailFilters {
    /// Filter by time range (created_at for most entities, effective_at for transactions)
    #[serde(flatten, default)]
    #[param(value_type = TimeRange)]
    pub time_range: Option<TimeRange>,
}

impl AuditTrailFilters {
    /// Validates the time range filter
    pub fn validate(&self) -> Result<(), String> {
        if let Some(time_range) = &self.time_range {
            if let Some(end_time) = time_range.end_time {
                if time_range.start_time > end_time {
                    return Err("start_time cannot be greater than end_time".to_string());
                }
            }
        }
        Ok(())
    }
}

/// Response for audit trail endpoint - chronologically sorted array of events
pub type AuditTrailResponse = Vec<AuditEvent>;

/// Audit event representing an activity in the system
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum AuditEvent {
    /// File uploaded successfully
    FileUploaded {
        /// Account information
        account: AccountData,

        /// Ingestion configuration ID
        #[schema(example = "ing_conf_123456")]
        ingestion_id: String,

        /// Ingestion history ID
        #[schema(example = "ing_hist_123456")]
        ingestion_history_id: String,

        /// File name or path
        #[schema(example = "transactions_2025_01_31.csv")]
        file_name: String,

        /// Timestamp when the file was uploaded
        #[serde(with = "common_utils::custom_serde::iso8601")]
        #[schema(value_type = String, example = "2025-01-31T18:30:00Z")]
        timestamp: time::PrimitiveDateTime,
    },

    /// Failed ingestion runs (grouped)
    IngestionsFailed {
        /// Account information
        account: AccountData,

        /// Number of failed ingestion runs
        #[schema(example = 2)]
        count: usize,

        /// Timestamp of the most recent failure
        #[serde(with = "common_utils::custom_serde::iso8601")]
        #[schema(value_type = String, example = "2025-01-31T18:30:00Z")]
        last_failed_at: time::PrimitiveDateTime,
    },

    /// Staging entries created
    StagingEntriesCreated {
        /// Account information
        account: AccountData,

        /// Number of entries created
        #[schema(example = 100)]
        count: usize,

        /// Timestamp of the most recent creation
        #[serde(with = "common_utils::custom_serde::iso8601")]
        #[schema(value_type = String, example = "2025-01-31T18:35:00Z")]
        timestamp: time::PrimitiveDateTime,
    },

    /// Staging entries that need manual review
    StagingEntryNeedsManualReview {
        /// Account information
        account: AccountData,

        /// Number of entries needing review
        #[schema(example = 5)]
        count: usize,

        /// Timestamp of the most recent entry needing review
        #[serde(with = "common_utils::custom_serde::iso8601")]
        #[schema(value_type = String, example = "2025-01-31T18:36:00Z")]
        timestamp: time::PrimitiveDateTime,
    },

    /// Expectations created (transactions with Expected status)
    ExpectationsCreated {
        /// Array of accounts involved in the expectations
        accounts: Vec<AccountData>,

        /// Number of expectations created
        #[schema(example = 50)]
        count: usize,

        /// Timestamp of the creation
        #[serde(with = "common_utils::custom_serde::iso8601")]
        #[schema(value_type = String, example = "2025-01-31T18:38:00Z")]
        timestamp: time::PrimitiveDateTime,
    },

    /// Transactions matched
    TransactionsMatched {
        /// Array of accounts involved in the matching
        accounts: Vec<AccountData>,

        /// Number of transactions matched
        #[schema(example = 100)]
        count: usize,

        /// Timestamp of the matching
        #[serde(with = "common_utils::custom_serde::iso8601")]
        #[schema(value_type = String, example = "2025-01-31T18:40:00Z")]
        timestamp: time::PrimitiveDateTime,
    },

    /// Transactions reconciled (moved to Posted status)
    TransactionsReconciled {
        /// Array of accounts involved in the reconciliation
        accounts: Vec<AccountData>,

        /// Number of transactions reconciled
        #[schema(example = 100)]
        count: usize,

        /// Timestamp of the reconciliation
        #[serde(with = "common_utils::custom_serde::iso8601")]
        #[schema(value_type = String, example = "2025-01-31T18:40:00Z")]
        timestamp: time::PrimitiveDateTime,
    },

    /// Transactions with mismatches detected
    TransactionsMismatched {
        /// Array of accounts involved in the mismatched transactions
        accounts: Vec<AccountData>,

        /// Number of mismatched transactions
        #[schema(example = 10)]
        count: usize,

        /// Timestamp of the mismatch detection
        #[serde(with = "common_utils::custom_serde::iso8601")]
        #[schema(value_type = String, example = "2025-01-31T18:41:00Z")]
        timestamp: time::PrimitiveDateTime,
    },
}
