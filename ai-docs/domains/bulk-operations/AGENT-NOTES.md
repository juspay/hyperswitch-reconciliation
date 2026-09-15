# Bulk Operations - Agent Notes

## Critical Rules
- **Maximum Records**: 500 records per bulk operation (MAX_BULK_RECORDS constant)
- **Concurrent Processing**: Uses `buffer_unordered(10)` for parallel execution
- **Profile Isolation**: All entities must belong to authenticated profile - cross-profile operations fail silently (returns "not found")
- **Idempotency**: Void operations on already-voided entities return `Skipped`, not `Failed`
- **Selection Deduplication**: Duplicate IDs in request are automatically deduplicated before processing

## Common Gotchas
<!-- AI-UPDATED: 2026-02-24 - Initial documentation -->
- **Result Status Mapping**:
  - `Success` = operation completed
  - `Skipped` = entity not eligible (wrong status, already processed)
  - `Failed` = entity not found OR operation error
- **ID vs Logical ID**:
  - StagingEntry uses `id` field for results
  - Transaction uses `transaction_id` (logical ID) for results, NOT the primary key `id`
- **Missing IDs**: When selecting by IDs, non-existent entities return `Failed { error: "not found" }` BEFORE processing starts
- **Filters Selection**: Uses existing filter types (ListStagingEntriesFilters, ApiTransactionAccountFilters) - same constraints apply
- **Concurrent Execution Order**: Results may not be in same order as input due to `buffer_unordered` - use `entity_id` to match results

## Design Decisions
- **Why Skipped vs Failed**:
  - `Skipped` indicates the entity exists but isn't eligible for the action (e.g., voiding an already-voided entity)
  - `Failed` indicates either the entity doesn't exist or an actual error occurred
  - This distinction helps operators understand whether action is needed
- **Why buffer_unordered(10)**:
  - Limits concurrent database operations to prevent overwhelming the system
  - 10 is a conservative default that balances throughput with resource usage
  - Each operation is independent - no ordering guarantees needed
- **Why Transaction Uses transaction_id**:
  - Results use logical `transaction_id` because transactions can have multiple versions
  - Primary key `id` is version-specific; `transaction_id` is stable across versions
  - Users identify transactions by logical ID, not internal version IDs

## Relationships
- **Staging Entries**: Bulk void operations call `staging_entry::void_staging_entry_core()`
- **Transactions**: Bulk void/manual_post call `transaction::void_transaction_core()` and `transaction::manual_post_transaction_core()`
- **Error Handling**: Shared error types in `errors/bulk_operations.rs`
- **Result Building**: Utility functions in `utils/bulk_operations.rs` for creating result items

## When Working Here
<!-- AI-UPDATED: 2026-02-24 - Adding new actions -->
- **Adding New Actions**:
  1. Add action variant to appropriate enum (StagingEntryBulkAction or TransactionBulkAction)
  2. Add handler in core/bulk_operations.rs matching function
  3. Use `futures::stream::iter().map().buffer_unordered(MAX_CONCURRENT_OPERATIONS).collect()` pattern
  4. Map results using utility functions: `success_result_item()`, `skipped_result_item()`, `failed_result_item()`
- **Adding New Selection Types**:
  - Add variant to StagingEntryBulkSelection or TransactionBulkSelection
  - Implement query logic in core function to fetch entities
  - Handle missing entities: pre-populate results with Failed items for not found IDs
- **Processing Pattern**:
  ```rust
  futures::stream::iter(entities.into_iter())
      .map(|entity| async move {
          match process_entity(entity).await {
              Ok(Some(_)) => success_result_item(id),
              Ok(None) => skipped_result_item(id, "not eligible"),
              Err(e) => failed_result_item(id, e.to_string()),
          }
      })
      .buffer_unordered(MAX_CONCURRENT_OPERATIONS)
      .collect::<Vec<_>>()
      .await
  ```

## Staging Entry Bulk Operations
<!-- AI-UPDATED: 2026-02-24 - Staging entry specifics -->
- **Endpoint**: `POST /staging_entries/bulk_operations`
- **Available Actions**: `Void { reason: Option<String> }`
- **Selection Methods**:
  - `Ids { ids: Vec<String> }` - specific staging entry IDs
  - `Filters { filters: ListStagingEntriesFilters }` - query-based selection
- **Void Eligibility**: Only staging entries in `Pending` or `NeedsManualReview` status are voidable
- **Result Entity ID**: Uses staging entry `id` field

## Transaction Bulk Operations
<!-- AI-UPDATED: 2026-02-24 - Transaction specifics -->
- **Endpoint**: `POST /transactions/bulk_operations`
- **Available Actions**:
  - `Void { reason: Option<String> }` - void eligible transactions
  - `ManualPost { reason: Option<String> }` - manually post matched transactions (transactions only)
- **Selection Methods**:
  - `Ids { ids: Vec<String> }` - specific transaction IDs
  - `Filters { filters: ApiTransactionAccountFilters }` - query-based selection
- **Void Eligibility**: Transactions in `Expected`, `Mismatched`, `PartiallyReconciled`, or `Posted` status
- **Manual Post Eligibility**: Only transactions in `MatchedForce` status (after force reconciliation)
- **Result Entity ID**: Uses `transaction_id` (logical ID), NOT primary key `id`

## ManualPost Gotchas
<!-- AI-UPDATED: 2026-02-24 - ManualPost specific behavior -->
- **Prerequisite Flow**: ManualPost requires prior force reconciliation
  - Create mismatched transaction (DataMismatch status)
  - Force reconcile to get MatchedForce status
  - ManualPost to convert to Posted with manual posted data
- **Status Transitions**:
  - `MatchedForce` -> `Posted(Manual)` after successful ManualPost
  - All entries get `EntryStatus::Posted`
- **Non-Eligible Returns Skipped**: Trying to ManualPost on Expected, Mismatched, or already Posted returns `Skipped`, not error
- **Audit Trail**: Reason field stored in transaction data for compliance

## Testing Gotchas
<!-- AI-UPDATED: 2026-02-24 - Testing patterns -->
- **Result Ordering**: Don't assume results are in same order as input - use `entity_id` to match
- **Map Helper**: Use `map_results_by_entity_id()` helper to create HashMap for easy lookup:
  ```rust
  fn map_results_by_entity_id(results: Vec<BulkOperationResultItem>) -> HashMap<String, BulkOperationResultItem> {
      results.into_iter().map(|r| (r.entity_id.clone(), r)).collect()
  }
  ```
- **Profile Isolation Tests**: Create entities with different profiles, verify cross-profile access returns "not found"
- **Duplicate ID Tests**: Send same ID multiple times, verify only one result returned
- **Non-Voidable Tests**: Void same entity twice - first should succeed, second should be skipped
- **ManualPost Flow Tests**: Must create mismatched transaction first, force reconcile, then manual post
- **Filter Tests**: When using Filters selection, results count depends on matching entities in database
