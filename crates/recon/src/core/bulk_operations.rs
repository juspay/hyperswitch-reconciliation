use api_models::bulk_operations::{
    BulkOperationResultItem, StagingEntryBulkAction, StagingEntryBulkOperationRequest,
    StagingEntryBulkSelection, TransactionBulkAction, TransactionBulkOperationRequest,
    TransactionBulkSelection,
};
use common_types::staging_entry::ListStagingEntriesFilters;
use common_utils::auth::AuthContext;
use diesel_models::{staging_entry::StagingEntry, transaction::Transaction};
use error_stack::{Report, ResultExt};
use futures::StreamExt;

use std::collections::HashSet;
use std::ops::Not;

use crate::core::{staging_entry, transaction};
use crate::db::transaction::TransactionInterface;
use crate::errors::bulk_operations::BulkOperationError;
use crate::routes::AppState;
use crate::types::{
    transformers::{ForeignFrom, ForeignInto},
    ApplicationResponse,
};
use crate::utils::bulk_operations as bulk_operation_utils;

pub const MAX_BULK_RECORDS: usize = 500;
pub const MAX_CONCURRENT_OPERATIONS: usize = 10;

pub async fn execute_staging_entry_bulk_operation(
    state: AppState,
    request: StagingEntryBulkOperationRequest,
    auth_context: AuthContext,
) -> Result<ApplicationResponse<Vec<BulkOperationResultItem>>, Report<BulkOperationError>> {
    let mut results = Vec::new();

    let staging_entries = match request.selection {
        StagingEntryBulkSelection::Ids { ids } => {
            let db_staging_entries = state
                .store
                .list_staging_entries_by_primary_ids_and_profile(
                    &ids,
                    &auth_context.profile_id,
                    Some(bulk_operation_utils::get_query_limit(MAX_BULK_RECORDS)?),
                )
                .await
                .change_context(BulkOperationError::InternalServerError)?;

            let found_ids: HashSet<String> = db_staging_entries
                .iter()
                .map(|entry| entry.id.clone())
                .collect();

            results.extend(ids.into_iter().filter_map(|id| {
                found_ids
                    .contains(&id)
                    .not()
                    .then_some(bulk_operation_utils::failed_result_item(
                        id,
                        "Staging Entry not found".to_string(),
                        None,
                    ))
            }));

            db_staging_entries
        }
        StagingEntryBulkSelection::Filters { filters } => state
            .store
            .list_staging_entries_by_profile_id(
                &auth_context.profile_id,
                ListStagingEntriesFilters::foreign_from(filters),
                Some(bulk_operation_utils::get_query_limit(MAX_BULK_RECORDS)?),
            )
            .await
            .change_context(BulkOperationError::InternalServerError)?
            .into_iter()
            .map(|view| view.staging_entry)
            .collect(),
    };

    if staging_entries.len() > MAX_BULK_RECORDS {
        return Err(Report::new(BulkOperationError::RecordLimitExceeded {
            max: MAX_BULK_RECORDS,
        }));
    }

    let (unique_staging_entries, _staging_entry_ids) = staging_entries.into_iter().fold(
        (Vec::new(), HashSet::new()),
        |(mut unique, mut staging_entry_ids), staging_entry| {
            if staging_entry_ids.insert(staging_entry.id.clone()) {
                unique.push(staging_entry);
            }
            (unique, staging_entry_ids)
        },
    );

    results.extend(match request.action {
        StagingEntryBulkAction::Void { reason } => {
            bulk_staging_entry_void_core(
                &state,
                unique_staging_entries,
                reason,
                Some(auth_context.user_id),
            )
            .await
        }
    });

    Ok(ApplicationResponse::json(results))
}

pub async fn execute_transaction_bulk_operation(
    state: AppState,
    request: TransactionBulkOperationRequest,
    auth_context: AuthContext,
) -> Result<ApplicationResponse<Vec<BulkOperationResultItem>>, Report<BulkOperationError>> {
    let mut results = Vec::new();

    let transactions = match request.selection {
        TransactionBulkSelection::Ids { ids } => {
            let db_transactions = state
                .store
                .list_transactions_by_primary_ids_and_profile(
                    &ids,
                    &auth_context.profile_id,
                    Some(bulk_operation_utils::get_query_limit(MAX_BULK_RECORDS)?),
                )
                .await
                .change_context(BulkOperationError::InternalServerError)?;

            let found_ids: HashSet<String> =
                db_transactions.iter().map(|txn| txn.id.clone()).collect();

            results.extend(ids.into_iter().filter_map(|id| {
                found_ids
                    .contains(&id)
                    .not()
                    .then_some(bulk_operation_utils::failed_result_item(
                        id,
                        "Transaction not found".to_string(),
                        None,
                    ))
            }));

            db_transactions
        }
        TransactionBulkSelection::Filters { filters } => state
            .store
            .list_with_accounts_and_entries(
                &auth_context.profile_id,
                filters.foreign_into(),
                Some(bulk_operation_utils::get_query_limit(MAX_BULK_RECORDS)?),
            )
            .await
            .change_context(BulkOperationError::InternalServerError)?
            .into_iter()
            .map(|view| view.transaction)
            .collect(),
    };

    if transactions.len() > MAX_BULK_RECORDS {
        return Err(Report::new(BulkOperationError::RecordLimitExceeded {
            max: MAX_BULK_RECORDS,
        }));
    }

    let (unique_transactions, _transaction_ids) = transactions.into_iter().fold(
        (Vec::new(), HashSet::new()),
        |(mut unique, mut transaction_ids), transaction| {
            if transaction_ids.insert(transaction.transaction_id.clone()) {
                unique.push(transaction);
            }
            (unique, transaction_ids)
        },
    );

    results.extend(match request.action {
        TransactionBulkAction::Void { reason } => {
            bulk_transaction_void_core(
                &state,
                unique_transactions,
                reason,
                Some(auth_context.user_id),
            )
            .await
        }
        TransactionBulkAction::ManualPost { reason } => {
            bulk_transaction_manual_post_core(
                &state,
                unique_transactions,
                reason,
                Some(auth_context.user_id),
            )
            .await
        }
    });

    Ok(ApplicationResponse::json(results))
}

async fn bulk_staging_entry_void_core(
    state: &AppState,
    staging_entries: Vec<StagingEntry>,
    reason: Option<String>,
    modified_by: Option<String>,
) -> Vec<BulkOperationResultItem> {
    futures::stream::iter(staging_entries)
        .map(|staging_entry| {
            let reason = reason.clone();
            let modified_by = modified_by.clone();
            async move {
                let entity_id = staging_entry.id.clone();
                let logical_id = staging_entry.staging_entry_id.clone();

                match staging_entry::void_staging_entry_core(
                    state,
                    staging_entry,
                    reason,
                    modified_by,
                )
                .await
                {
                    Ok(Some(_)) => bulk_operation_utils::success_result_item(entity_id, logical_id),
                    Ok(None) => bulk_operation_utils::skipped_result_item(
                        entity_id,
                        "Staging entry not voidable".to_string(),
                        logical_id,
                    ),
                    Err(error) => bulk_operation_utils::failed_result_item(
                        entity_id,
                        error.current_context().to_string(),
                        Some(logical_id),
                    ),
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT_OPERATIONS)
        .collect::<Vec<_>>()
        .await
}

async fn bulk_transaction_void_core(
    state: &AppState,
    transactions: Vec<Transaction>,
    reason: Option<String>,
    modified_by: Option<String>,
) -> Vec<BulkOperationResultItem> {
    futures::stream::iter(transactions)
        .map(|transaction| {
            let reason = reason.clone();
            let modified_by = modified_by.clone();
            async move {
                let entity_id = transaction.id.clone();
                let logical_id = transaction.transaction_id.clone();

                match transaction::void_transaction_core(state, transaction, reason, modified_by)
                    .await
                {
                    Ok(Some(_)) => bulk_operation_utils::success_result_item(entity_id, logical_id),
                    Ok(None) => bulk_operation_utils::skipped_result_item(
                        entity_id,
                        "Transaction not voidable".to_string(),
                        logical_id,
                    ),
                    Err(error) => bulk_operation_utils::failed_result_item(
                        entity_id,
                        error.current_context().to_string(),
                        Some(logical_id),
                    ),
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT_OPERATIONS)
        .collect::<Vec<_>>()
        .await
}

async fn bulk_transaction_manual_post_core(
    state: &AppState,
    transactions: Vec<Transaction>,
    reason: Option<String>,
    modified_by: Option<String>,
) -> Vec<BulkOperationResultItem> {
    futures::stream::iter(transactions)
        .map(|transaction| {
            let reason = reason.clone();
            let modified_by = modified_by.clone();
            async move {
                let entity_id = transaction.id.clone();
                let logical_id = transaction.transaction_id.clone();

                match transaction::manual_post_transaction_core(
                    state,
                    transaction,
                    reason,
                    modified_by,
                )
                .await
                {
                    Ok(Some(_)) => bulk_operation_utils::success_result_item(entity_id, logical_id),
                    Ok(None) => bulk_operation_utils::skipped_result_item(
                        entity_id,
                        "Transaction not eligible for manual posting".to_string(),
                        logical_id,
                    ),
                    Err(error) => bulk_operation_utils::failed_result_item(
                        entity_id,
                        error.current_context().to_string(),
                        Some(logical_id),
                    ),
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT_OPERATIONS)
        .collect::<Vec<_>>()
        .await
}
