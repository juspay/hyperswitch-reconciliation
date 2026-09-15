use api_models::bulk_operations::{BulkOperationResultItem, BulkResultStatus};
use error_stack::ResultExt;

use crate::errors::bulk_operations::{BulkOperationError, BulkOperationResult};

pub fn success_result_item(primary_id: String, logical_id: String) -> BulkOperationResultItem {
    BulkOperationResultItem {
        primary_id,
        status: BulkResultStatus::Success { logical_id },
    }
}

pub fn skipped_result_item(
    primary_id: String,
    reason: String,
    logical_id: String,
) -> BulkOperationResultItem {
    BulkOperationResultItem {
        primary_id,
        status: BulkResultStatus::Skipped { reason, logical_id },
    }
}

pub fn failed_result_item(
    primary_id: String,
    error: String,
    logical_id: Option<String>,
) -> BulkOperationResultItem {
    BulkOperationResultItem {
        primary_id,
        status: BulkResultStatus::Failed { error, logical_id },
    }
}

pub fn get_query_limit(max_limit: usize) -> BulkOperationResult<i64> {
    i64::try_from(max_limit + 1).change_context(BulkOperationError::InternalServerError)
}
