# Audit Trail - Agent Notes

## What is Audit Trail?

The audit trail system provides a chronological, profile-scoped view of system activities. It aggregates events from ingestion, transformation, and reconciliation processes into a unified timeline.

**Key files:**
- `routes/audit_trail.rs:31` - GET /audit_trail endpoint
- `core/audit_trail.rs:13` - Main business logic
- `utils/audit_trail.rs:23` - Event fetching functions
- `domain/audit_trail.rs:8` - Domain models and grouping logic
- `api_models/audit_trail.rs:34` - AuditEvent enum (API response)

## Critical Gotchas

### Three Distinct Grouping Strategies
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- **FileUploaded**: Never groups - each file upload is a separate event
- **IngestionsFailed**: Consecutive grouping only - merges only if events appear consecutively in sorted list
- **All others** (StagingEntriesCreated, ExpectationsCreated, TransactionsReconciled, etc.): TimeWindow grouping - merges events within 5-minute window even if non-consecutive

```rust
// domain/audit_trail.rs:119
pub fn grouping_strategy(&self) -> GroupingStrategy {
    match self {
        Self::FileUploaded { .. } => GroupingStrategy::None,
        Self::IngestionFailed { .. } => GroupingStrategy::Consecutive,
        Self::StagingEntryCreated { .. } | /* others */ => GroupingStrategy::TimeWindow,
    }
}
```

### TimeWindow Grouping Scans All Previous Events
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- TimeWindow grouping iterates through **all** previous grouped events in reverse
- Can merge events that are far apart in the list if within time window
- Performance consideration: O(n²) worst case if many ungrouped events within window
- See `utils/audit_trail.rs:238-248` for the fold logic

### Grouped Events Store LAST Timestamp
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- When merging occurrences, the timestamp is **always updated** to the latest occurrence
- For grouped events, timestamp represents the most recent activity, not the first
- Example: IngestionsFailed shows `last_failed_at`, not `first_failed_at`
- See `domain/audit_trail.rs:181-262` merge_occurrence() implementations

### Double Sorting Pattern
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- Occurrences sorted **before** grouping at `core/audit_trail.rs:68`
- Final events sorted **after** grouping at `core/audit_trail.rs:78`
- Second sort is necessary because grouping updates timestamps to last occurrence
- Both sorts use the same timestamp key for consistency

### Account Data Bulk Fetch Pattern
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- All account_ids collected from events **before** any account lookups
- Single bulk query at `core/audit_trail.rs:47-51` prevents N+1 queries
- Account map created for O(1) lookups during event transformation
- Missing accounts cause InternalServerError (no graceful degradation)

```rust
// core/audit_trail.rs:39-51
let unique_account_ids: Vec<String> = all_occurrences
    .iter()
    .flat_map(|occurrence| occurrence.account_ids())
    .collect::<HashSet<_>>()
    .into_iter()
    .collect();

let accounts = state
    .store
    .list_accounts_by_profile_id_and_account_ids(profile_id, &unique_account_ids)
    .await
```

### Discarded Items Filtered at Query Level
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- Ingestion: Uses status filter (Processed, Failed only) - excludes Discarded status
- Staging entries: Uses status filter + runtime filter for discarded_at
- Transactions: Runtime filter only (`group.transaction.discarded_at.is_none()`)
- Grouping logic never sees discarded items - they're filtered earlier

## Architecture Patterns

### Occurrence → Grouped → AuditEvent Pipeline
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
1. **Fetch**: Separate functions get occurrences from ingestion, staging, transactions
2. **Sort**: All occurrences sorted by timestamp
3. **Group**: Fold operation applies grouping strategies
4. **Transform**: GroupedAuditEvent → AuditEvent with account data
5. **Sort again**: Final events sorted by timestamp

### Routes → Core → Utils Layering
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- **routes/audit_trail.rs**: JWT auth, query parsing, API wrapper
- **core/audit_trail.rs**: Orchestrates pipeline, bulk fetching, transformation
- **utils/audit_trail.rs**: Domain functions for fetching events from different sources
- **domain/audit_trail.rs**: Domain models, grouping logic, transformations

Note: This is slightly different from typical Routes → Core → DB pattern - utils layer contains domain logic for event fetching.

## Event Types and Sources

### Ingestion Events
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- **FileUploaded**: Created from IngestionHistory with status=Processed
  - Requires ingestion_history.data to contain file_name
  - Timestamp: ingestion_history.created_at
  - Single account per event
- **IngestionsFailed**: Created from IngestionHistory with status=Failed
  - Groups consecutive failures only
  - Timestamp: last failure's created_at
  - Single account per event

### Staging Entry Events
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- **StagingEntriesCreated**: Created from staging_entry with status=Pending or Processed
  - Groups within 5-minute time window
  - Timestamp: staging_entry.created_at
- **StagingEntryNeedsManualReview**: Created from staging_entry with status=NeedsManualReview
  - Groups within 5-minute time window
  - Timestamp: staging_entry.created_at

### Transaction Events
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- **ExpectationsCreated**: Transactions with status=Expected
- **TransactionsReconciled**: Transactions with status=Posted
- **TransactionsMismatched**: Transactions with status=Mismatched
- All use transaction.created_at as timestamp
- All group within 5-minute time window
- Can involve multiple accounts (collected from entries)
- Account IDs are sorted for consistent ordering

## Common Pitfalls

### Don't Assume First Timestamp
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- Grouped events always show the **last** occurrence timestamp
- If you need to know when a group started, you'll need additional tracking
- This is by design - shows "as of" time for the entire group

### Don't Mix Grouping Strategies
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- Each event type has a fixed grouping strategy
- You can't change grouping behavior without modifying domain logic
- TimeWindow constant is 5 minutes - see `domain/audit_trail.rs:8`

### Account Map Failures are Fatal
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- If account is missing from map during transformation, request fails
- No partial results - entire audit trail request returns InternalServerError
- Ensure accounts exist before creating events, or handle orphaned events

### TimeRange Filtering Timing
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- time_range filter applied at query level for each entity type
- For transactions, filters by effective_at (transaction timestamp)
- For ingestion and staging, filters by created_at
- Grouping happens after filtering - can't group across filter boundaries

## Testing Considerations

### Grouping Logic Tests
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- Test consecutive vs non-consecutive occurrences
- Test time window boundaries (exactly 5 minutes, just over, just under)
- Test that FileUploaded never groups even when identical
- Test that IngestionsFailed doesn't group if interrupted by another event type

### Edge Cases to Test
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- Empty results from each source (ingestion, staging, transactions)
- Missing account data (should fail gracefully with error)
- Events at exact same timestamp (sort stability)
- Very large number of events (grouping performance)
- Discarded items mixed with active items

## Transaction Modified-By Tracking
<!-- AI-UPDATED: 2026-03-12 - Added modified_by column for user audit trail -->

### Database Schema
- Transaction table has `modified_by VARCHAR(64)` nullable column

### User vs System Operations
- **User-initiated**: `modified_by = Some(user_id)` from `auth_context.user_id`
  - Manual transaction creation
  - Void transaction operations
  - Manual post operations
  - Exception management flows (force reconcile, entry replacement)
  - Bulk void/manual post operations
- **System-automated**: `modified_by = None`
  - Auto-reconciliation via recon engine
  - Ingestion service automated transactions
  - Staging entry auto-promotion

### Critical Gotchas
- **Version semantics**: `modified_by` tracks who created the **new version**, not the original transaction creator
- **Bulk operations**: Each transaction in bulk void/manual post gets same `modified_by` value

## When Working Here

### Adding New Event Types
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
1. Add variant to `AuditEvent` enum in api_models/audit_trail.rs
2. Create Occurrence struct in domain/audit_trail.rs
3. Add variant to `AuditEventOccurrence` enum
4. Add variant to `GroupedAuditEvent` enum
5. Implement merge logic in `GroupedAuditEvent::merge_occurrence()`
6. Implement transformation in `GroupedAuditEvent::to_api_response()`
7. Add From impl for `AuditEventOccurrence` → `GroupedAuditEvent`
8. Create fetch function in utils/audit_trail.rs
9. Update core/audit_trail.rs to call new fetch function
10. Choose grouping strategy in `AuditEventOccurrence::grouping_strategy()`

### Modifying Grouping Logic
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- TimeWindow constant at `domain/audit_trail.rs:8`
- Grouping strategies at `domain/audit_trail.rs:11`
- Core fold logic at `utils/audit_trail.rs:216-253`
- Be careful: changing from TimeWindow to Consecutive (or vice versa) affects API behavior

### Performance Optimization
<!-- AI-UPDATED: 2025-11-11 - Initial documentation -->
- Bulk account fetching is critical - never fetch accounts in a loop
- Consider adding indices on created_at for each source table
- TimeWindow grouping is O(n²) worst case - consider limiting window size
- Current implementation loads all events into memory - may need pagination for large datasets
