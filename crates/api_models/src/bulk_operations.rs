use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    staging_entry::ApiListStagingEntriesFilters, transaction::ApiTransactionAccountFilters,
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StagingEntryBulkOperationRequest {
    pub action: StagingEntryBulkAction,
    pub selection: StagingEntryBulkSelection,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StagingEntryBulkAction {
    Void { reason: Option<String> },
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "selection_type", rename_all = "snake_case")]
pub enum StagingEntryBulkSelection {
    Ids {
        ids: Vec<String>,
    },
    Filters {
        filters: ApiListStagingEntriesFilters,
    },
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionBulkOperationRequest {
    pub action: TransactionBulkAction,
    pub selection: TransactionBulkSelection,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionBulkAction {
    Void { reason: Option<String> },
    ManualPost { reason: Option<String> },
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "selection_type", rename_all = "snake_case")]
pub enum TransactionBulkSelection {
    Ids {
        ids: Vec<String>,
    },
    Filters {
        filters: ApiTransactionAccountFilters,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BulkResultStatus {
    Success {
        logical_id: String,
    },
    Skipped {
        reason: String,
        logical_id: String,
    },
    Failed {
        error: String,
        logical_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct BulkOperationResultItem {
    pub primary_id: String,
    #[serde(flatten)]
    pub status: BulkResultStatus,
}
