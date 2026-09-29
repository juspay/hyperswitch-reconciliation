# Transactions - Agent Notes

> **Note**: Also read `OVERVIEW-API.md` when changing profile overview aggregation, and
> `MANUAL-RECONCILIATION.md` when changing either manual reconciliation endpoint.

## Critical Rules
- **Entry Array Linking**: Transactions link multiple entries via `entry_id` array field
- **Atomic Operations**: All entries in a transaction must be processed together
- **Balance Validation**: Direct transaction API now rejects unbalanced transactions with 400 error (`UnbalancedTransaction`)
  - Mismatched transactions only created through ProcessAsTransaction → ProcessAsConfirmation flow
  - Context: Transaction creation validates credit_sum = debit_sum, confirmation processing handles mismatches
- **Version Control**: Uses `version` field for optimistic locking
- **Entry Version Field**: All entries now include `version: i32` field (default 1) for optimistic locking
- **Soft Delete Pattern**: Uses `discarded_at` and `discarded_status` fields
- **Search Expectations**: Expected entries automatically create search_expectations records for reconciliation
- **Status Determination**: Transaction status is determined server-side with priority logic:
  - If unbalanced (credits ≠ debits) → Transaction status = `Mismatched` (highest priority)
  - Else if ANY Expected entries exist → Transaction status = `Expected`
  - Else (balanced, only Direct entries) → Transaction status = `Posted`
- **Mixed Entry Types**: Transactions CAN contain both Direct and Expected entries
- **Search Expectation Status**: Search expectations use ExpectationStatus enum (Expected, Archived)
- **Reconciliation Integration**: Search expectations link expected entries to reconciliation rules
- **Double-Entry Implementation**: BalanceDirection (Credit/Debit) implements double-entry accounting
<!-- AI-UPDATED: 2025-06-30 - ProcessAsTransaction flow integration -->
- **Automatic Creation from Staging Entries**: Transactions can be created automatically via ProcessAsTransaction flow in recon engine
- **Search Expectations in ProcessAsTransaction**: Expected entries created during recon processing automatically generate search expectations with rule_id reference

## Common Gotchas
- **Array Storage**: `entry_id` is stored as `Vec<String>` using `DieselArray` wrapper
- **Transaction vs Entry Status**: Both have independent status fields that must be coordinated
- **Entry Status Logic**: Direct entry status depends on transaction balance and entry types:
  - Unbalanced transactions → Direct entries get `EntryStatus::Mismatched`
  - Balanced with Expected entries → Direct entries get `EntryStatus::Pending`
  - Balanced with only Direct entries → Direct entries get `EntryStatus::Posted`
  - Expected entries ALWAYS get `EntryStatus::Expected` regardless of balance
- **Amount Storage**: Transactions now store credit_amount and debit_amount fields (MinorUnit) to track variance, plus currency field. These are calculated from individual entries during transaction creation
- **Amount Sign**: Individual entries store amounts using MinorUnit, business logic validates only positive amounts are accepted
- **Automatic Amount Calculation in Versioning**: `new_versioned()` constructor automatically calculates credit/debit amounts by iterating through provided entries and summing by `BalanceDirection`
- **ID Formats**: Two IDs exist - `id` (primary key) and `transaction_id` (business identifier)
- **Profile Scoping**: Transactions are scoped to profile_id like all other entities
<!-- AI-UPDATED: 2026-07-30 - Mismatch reason data location and shape constraint -->
- **Mismatch reason data lives in `data`, not `status`**: `TransactionData::DataMismatch(MetadataMismatchData)` (the `data` field) carries `mismatched_fields`; `ApiTransactionStatus::DataMismatch`/`DomainTransactionStatus::DataMismatch` (`status`/`sub_status`) are bare on purpose.
- **`TransactionDataV3::DataMismatch`'s payload must be the plain `MetadataMismatchDataV1` struct, not the versioned `MetadataMismatchData` enum wrapper**: `TransactionDataV3` is internally tagged, so an old persisted row (`{"status":"data_mismatch"}`, no other keys) can't supply a version tag for the wrapper enum and would fail to deserialize. `#[serde(default)]` only defaults a missing struct field, not a missing enum-variant tag.
<!-- AI-UPDATED: 2026-05-05 - construction order: entries → transaction → search_expectations -->
- **Construction order for `create_transaction`**: build entries first, build the transaction second, build search expectations third. Each function self-allocates the ids it owns:
  - `Vec<TransactionEntry>::get_new_entries(merchant_id, encryptor) -> (Vec<EntryNew>, DieselTransactionData)` — self-allocates the `logical_transaction_id` (`txn_main_*`) and stamps it on every `EntryNew`.
  - `TransactionNew::new(&entries, profile_id, rule_id, data, modified_by)` — self-allocates `id` (`txn_*`); reads `transaction_id` back from `entries[0].logical_transaction_id`. Hardcodes version=1 (versioning uses `new_versioned` instead).
  - `Vec<TransactionEntry>::get_new_search_expectations(merchant_id, encryptor, primary_transaction_id) -> Vec<SearchExpectationNew>` — caller passes `transaction_new.id.clone()` as `primary_transaction_id` so the FK matches.
  - **Why split entries / search_expectations into separate functions**: at the DB level, search_expectations link to the transaction via `primary_transaction_id`, not to entries. Building them after `TransactionNew` self-allocates its id removes any second id source — the FK-mismatch bug class (where two independent `generate_id_with_default_len("txn")` calls produced divergent ids that look fine on insert but return empty on lookup) is now architecturally impossible.
- **`TransactionNew::new_versioned` self-allocates id internally**: signature is `(previous, update, modified_by)`, no `id` parameter. Safe because the surviving versioning flows (`manual_void_transaction_core`, `manual_post_transaction_core`, `force_reconcile_transaction`) do not create new search_expectations alongside the new transaction version. **If you add a new versioning flow that needs to create search_expectations, do not reuse this pattern** — you'd need to thread `new_txn.id.clone()` through to a separate `SearchExpectationNew` builder, or restructure to use `TransactionNew::new` instead.
- **Extension Trait Pattern**: `TransactionNewExt::new()` in `recon/utils/transaction.rs` builds a `TransactionNew` from `&[EntryNew]`, validating same-currency across entries and summing credit/debit amounts before constructing the diesel struct.
  - **Why an extension trait**: diesel_models cannot import recon utilities, so amount-calculation + currency-mismatch validation lives in recon as an `impl TransactionNewExt for TransactionNew`
- **Currency Consistency**: Transaction currency is taken from first entry - all entries in a transaction must have same currency (validated at API layer)
- **Error Context Preservation**: Database transaction closures require `DatabaseError` type, but validation errors should be caught BEFORE the transaction to preserve specific error types (AccountNotFound, InvalidAmount) for proper HTTP status codes (404, 400) instead of generic 500 errors
- **Validation-First Error Pattern**: Perform all business logic validation (positive amounts, account existence, currency matching, balance validation) BEFORE entering database transaction to ensure proper error types are thrown instead of generic `DatabaseError::Others`
- **Interface Design**: TransactionInterface only includes read operations (find, list) since transaction creation requires atomic operations within a database transaction closure that can't use the interface's connection-getting methods
- **Search Expectations for Mismatched**: Expected entries in mismatched transactions still create search expectations with `ExpectationStatus::Expected` - the reconciliation system can handle these once the balance is corrected
<!-- AI-UPDATED: 2025-07-03 - Effective timestamp implementation -->
- **Effective Timestamp Required**: All transaction entries (Direct and Expected) MUST include effective_at field (TIMESTAMP type)
- **Effective Timestamp in API**: Transaction API uses `time::PrimitiveDateTime` with ISO format (YYYY-MM-DDTHH:MM:SSZ) for effective_at field
<!-- AI-UPDATED: 2025-07-16 - TransactionAccountView and flattened responses -->
- **Version Field Required**: All entries now require `version: i32` parameter in creation functions (defaults to 1)
- **Flattened Response Structures**: New API models provide flattened transaction+entry+account data:
  - `TransactionWithEntriesResponse`: Transaction with embedded `EntryData` array
  - `EntryData`: Entry with embedded `AccountData`, including `status` (EntryStatus) and `amount` (AmountResponse) for complete entry information
  - `AccountData`: Complete account information for flattened responses
- **TransactionAccountFilters**: New filter struct in `common_types::transaction` for complex queries
  - Supports filtering by `transaction_status`, `rule_id`, and `time_range`
  - Provides `credit_account` and `debit_account` filters to search for transactions with specific credit or debit entries. These accept comma-separated account IDs.
  - Uses `#[serde(flatten)]` for `time_range` to expose `start_time` and `end_time` as direct query parameters.
<!-- AI-UPDATED: 2025-01-16 - API response simplification -->
<!-- AI-UPDATED: 2026-03-31 - modified_by user conversion centralized -->
- **Direct Array Response**: Transaction list API returns `Vec<TransactionWithEntriesResponse>` directly without wrapper
  - Removed `TransactionListResponse` wrapper struct for cleaner API
  - Frontend can directly iterate over array response
- **GroupedTransactions Pattern**: Internal helper struct for cleaner transaction grouping
  - Encapsulates transaction + entries + rule data
  - Provides `to_response()` method for conversion to API response
  - Separates grouping logic from response transformation

## Design Decisions
- **Why Array of Entry IDs**:
  - Supports variable number of entries per transaction
  - Enables complex multi-account transactions
  - Maintains referential integrity at app level
- **Why Separate Status Fields**:
  - Transaction status reflects overall state
  - Entry status tracks individual line items
  - Enables partial transaction processing
- **Why Version Field**:
  - Prevents concurrent modification issues
  - Supports optimistic locking pattern
  - Audit trail for changes

## Relationships
- **Entries**: One-to-many via entry_id array - must maintain consistency
- **Accounts**: Indirect through entries - one transaction can affect multiple accounts
- **Business Profile**: Direct relationship via profile_id
- **Currency**: Single currency per transaction (multi-currency requires multiple transactions)

## When Working Here
- **Creating Transactions**:
  - Validate minimum 2 entries required (error: InsufficientEntries)
  - Validate account existence and profile ownership per entry (error: AccountNotFound)
  - Validate positive amounts for all entries (error: InvalidAmount)
  - Determine transaction status based on entry types (Expected entries take priority)
  - Set initial version to 1
  - Generate both `id` and `transaction_id` server-side
<!-- AI-UPDATED: 2025-06-30 - ProcessAsTransaction flow -->
- **Creating Transactions via Recon Engine**:
  - ProcessAsTransaction flow creates transactions automatically from staging entries
  - Main entry (Direct) created from staging entry
  - Expected entries created for each rule target with opposite BalanceDirection
  - Search expectations automatically generated linking source to target accounts
  - All operations atomic - rollback on any failure
  - Transaction ID format: `txn_main_{nanoid}` for recon-created transactions
<!-- AI-UPDATED: 2025-09-29 - Enhanced match rule validation with detailed mismatch types -->
- **Match Rule Validation Enhancement**:
  - `MatchRuleValidation` now returns specific `MismatchType` instead of boolean
  - Detailed mismatch information: `AmountMismatch`, `BalanceDirectionMismatch`, `CurrencyMismatch`, `MetadataMismatch`
  - Entry data field stores specific mismatch details with variance calculations
  - ProcessAsConfirmation flow creates entries with detailed mismatch context
  - Backward compatible: business logic behavior unchanged, only data richness improved
- **Updating Transactions**:
  - Check version hasn't changed (optimistic lock)
  - Increment version on update
  - Coordinate status with entry statuses
  - Never modify entry_id array after creation (except during versioning - new versions get new entry IDs)
- **Querying Transactions**:
  - Always filter by profile_id
  - Include entry details when needed
  - Handle soft-deleted records appropriately
<!-- AI-UPDATED: 2025-01-07 - Database query optimization for transaction_id filtering -->
- **List by Transaction ID**:
  - Use `list_by_profile_id` with optional `transaction_id` parameter
  - Database filters by both `profile_id` and `transaction_id` when provided
  - Results ordered by version DESC when filtering by transaction_id
  - More efficient than post-query filtering in application code
<!-- AI-UPDATED: 2025-01-21 - Security validation patterns for updates -->
- **Security Validation for Updates**:
  - Always use logical transaction_id from URL path, never accept database primary keys from client
  - Validate profile ownership by checking transaction belongs to authenticated profile
  - Pattern: Find transaction by profile_id + transaction_id combination for security
  - Error handling: Return 404 for both non-existent and unauthorized transactions (same error)
<!-- AI-UPDATED: 2025-01-21 - ProcessingErrorType consolidation pattern -->
- **Error Handling Consolidation**: ProcessingErrorType enum now includes error details inline
  - Pattern: Enum variants contain error context directly instead of separate error structures
  - Simplification: Single enum for all processing errors rather than multiple nested types
  - Usage: Store processing errors directly in JSONB data field during error scenarios
  - Benefit: Cleaner error handling and storage without complex nested structures
<!-- AI-UPDATED: 2025-01-22 - Void transaction implementation gotchas -->
- **Void Transaction Gotchas**:
  - **Search Expectation Filtering**: Only entries with `EntryStatus::Expected` have search expectations - filter before archiving
  - **Entry Version Consistency**: All entries in same transaction share same version number for atomic operations
  - **Archive vs Void Status**: Original transaction → `Archived`, new transaction → `Void` (different statuses)
  - **Logical ID Preservation**: New void transaction keeps same `transaction_id` but gets new primary key `id`
  - **Atomic Operation Order**: Archive original data BEFORE creating new void records to prevent inconsistent state
  - **Search Expectation Batch Update**: Use `batch_update_status_by_primary_transaction_ids()` for efficient bulk archiving
  - **Database Transaction Required**: All void operations MUST be wrapped in `conn.transaction_async()` for consistency
  - **Profile Authorization Pattern**: Always check `existing_transaction.profile_id != auth_context.profile_id` for security
  - **Entry Object Pattern**: Pass complete Entry objects to `TransactionVersionUpdate` - constructor handles ID extraction and amount calculation
<!-- AI-UPDATED: 2025-09-04 - Centralized archival function implementation -->
- **Centralized Archival Function**: `discard_transaction_with_entries_and_expectations()` in TransactionInterface
  - **Purpose**: Single function handling all archival operations for transaction, entries, and search expectations
  - **Used By**: Both `void_transaction()` and `process_confirmation_context()` functions
  - **Benefits**: Eliminates code duplication, ensures consistent archival logic, improves maintainability
  - **Operations**: Archives transaction → Archives entries → Archives search expectations (all atomic)
  - **Returns**: Tuple of (archived_transaction, archived_entries, archived_expectations) for caller use
  - **Error Handling**: Proper error context preservation with detailed logging for each operation
  - **Database Context**: Takes connection parameter, designed for use within existing database transactions

## TransactionAccountView Pattern
<!-- AI-UPDATED: 2025-07-16 - N+1 query elimination pattern -->
- **Problem Solved**: Eliminates N+1 queries when fetching transaction + entry + account data together
- **Single JOIN Query**: `TransactionAccountView` provides flattened data in one database call
- **Performance Benefits**: 
  - Single database round trip instead of 1+N queries
  - Database-optimized JOIN execution
  - Reduced latency and better caching
  - Consistent data from single transaction snapshot
- **When to Use**: Dashboard queries, reports, API endpoints returning combined data
- **When NOT to Use**: Simple queries needing only transaction or account data separately

## Code Organization Patterns
<!-- AI-UPDATED: 2025-01-16 - Transaction grouping refactor -->
- **Transaction Grouping Helper**: `group_transactions()` in `utils/transaction.rs`
  - Takes `Vec<TransactionAccountView>` and returns `Vec<GroupedTransactions>`
  - Uses `fold()` with HashMap for efficient grouping by (transaction_id, version)
  - Avoids error propagation in grouping logic - errors only in `to_response()`
  - Cleaner separation: grouping logic separate from response transformation
- **GroupedTransactions Struct**: Intermediate representation for grouped data
  - Contains: transaction, entries vec, and rule data
  - Single responsibility: `to_response()` handles all conversions and validations
  - Amount conversions (MinorUnit to AmountResponse) happen only during response generation
  - Error context preservation through proper error mapping

## Testing Gotchas
- **Entry Type Consistency**: Test mixed Direct/Expected entries are supported (both types in same transaction)
- **Minimum Entry Count**: Test single entry or empty array is rejected (400 error)
- **Account Validation**: Test invalid account IDs are rejected (404 error)
- **Amount Validation**: Test positive amounts only - negative amounts would be rejected
- **Entry Processing**: Test all entries are created atomically (all succeed or all fail)
- **Authorization**: Test entries can only be created for accounts in authenticated profile
- **ID Generation**: Test transaction_id and entry_ids are generated server-side
- **Balance Validation**: Test that total debits must equal total credits
<!-- AI-UPDATED: 2025-01-22 - Void transaction testing requirements -->
- **Void Transaction Testing**:
  - **Version Incrementing**: Test void transaction has `version = original.version + 1`
  - **Archive Status**: Test original transaction status changes to `Archived` after void
  - **Search Expectation Cleanup**: Test search expectations for Expected entries are archived
  - **Cross-Profile Security**: Test voiding transaction from different profile returns 404
  - **Atomic Rollback**: Test partial failures rollback entire void operation
  - **Data Preservation**: Test original transaction data preserved in `discarded_data` field
  - **Void Reason Storage**: Test void reason correctly stored in new transaction's data field
  - **Entry Status Consistency**: Test all new void entries have `EntryStatus::Void` status
  - **Logical ID Consistency**: Test void transaction keeps same `transaction_id` as original
  - **Entry ID Array Updates**: Test void transaction `entry_id` array references NEW entry IDs, not original ones
  - **Amount Recalculation**: Test void transaction amounts recalculated from new entries, not inherited from original

## Query Filters
<!-- AI-UPDATED: 2025-01-07 - Added query filters for entries and staging entries -->
<!-- AI-UPDATED: 2025-01-16 - Fixed Swagger/OpenAPI type specifications -->
- **Entry Filters**: ListEntriesFilters supports:
  - `status`: Comma-separated list of EntryStatus values (e.g., `?status=pending,posted`)
  - `entry_type`: Comma-separated list of BalanceDirection values (e.g., `?entry_type=credit,debit`) — `Option<Vec<BalanceDirection>>` in this struct (the newer `POST /entries/list` deliberately differs: single `Option<BalanceDirection>`)
  - `time_range`: Flattened fields `start_time` and optional `end_time` (e.g., `?start_time=2024-01-01T00:00:00Z&end_time=2024-12-31T23:59:59Z`)
- **Staging Entry Filters**: ListStagingEntriesFilters supports:
  - `status`: Comma-separated list of `ApiStagingEntryCoarseStatus` values (e.g., `?status=pending,processed`). Coarse deliberately — a query param needs `FromStr`, and the query fans each value out to its granular DB statuses. See the "Staging Entry Implementation Details" section on the 2-step status swap.
  - `entry_type`: Comma-separated list of BalanceDirection values (e.g., `?entry_type=credit,debit`) — `Option<Vec<BalanceDirection>>` in this struct (the newer `POST /entries/list` deliberately differs: single `Option<BalanceDirection>`)
  - `currency`: Comma-separated list of Currency values (e.g., `?currency=USD,EUR`)
  - `time_range`: Flattened fields `start_time` and optional `end_time` (same format as entries)
- **Transaction Filters**: TransactionAccountFilters supports:
  - `transaction_id`: Optional specific transaction ID filter
  - `transaction_status`: Comma-separated list of TransactionStatus values (e.g., `?transaction_status=posted,pending`)
  - `account_id`: Comma-separated list of account IDs (e.g., `?account_id=account_123,account_456`)
  - `entry_type`: Comma-separated list of BalanceDirection values (e.g., `?entry_type=credit,debit`) — `Option<Vec<BalanceDirection>>` in this struct (the newer `POST /entries/list` deliberately differs: single `Option<BalanceDirection>`)
  - `rule_id`: Optional specific rule ID filter
  - `time_range`: Same flattened time range fields as other filters
- **Filter Implementation**: 
  - All filters use `#[serde(flatten, default)]` for time_range to expose start_time/end_time as direct query parameters
  - `TimeRange` struct now derives `IntoParams` to properly expose fields in OpenAPI/Swagger
  - All filter structs use proper `#[param(value_type = ...)]` annotations for correct Swagger type display
  - Using `#[serde(default)]` on optional fields eliminates the need for `required = false` in param annotations
- **Custom Deserializers**: Status, currency, and account_id filters use `parse_comma_separated` from `common_utils::custom_serde::query_parsers`
- **Swagger Improvements**: 
  - Fixed all `value_type` specifications to use actual enum types instead of generic `String` or `Vec<String>`
  - Added missing schema components (TimeRange, BalanceDirection, TransactionStatus, etc.) to route OpenAPI definitions
  - Ensures Swagger UI displays proper enum values and field types instead of generic strings
<!-- AI-UPDATED: 2025-07-11 - Profile scoping for staging entries -->
- **Staging Entry Profile Scoping**: 
  - Staging entries now require `profile_id` field (migration added)
  - New endpoint `GET /staging_entries` lists all staging entries for authenticated profile
  - Existing `GET /accounts/{account_id}/staging_entries` still available for account-specific queries
  - All file upload flows automatically set profile_id from auth context

## Staging Entry Implementation Details
<!-- AI-UPDATED: 2025-07-21 - Staging entry ID format and transformation tracking -->
- **ID Format**: Staging entry IDs use format `"stag_ent_{nanoid(10)}"` - NOT `"staging_entry_"`
  - **Critical for Tests**: Test assertions must use `starts_with("stag_ent_")` not `starts_with("staging_entry_")`
  - Generated in `StagingEntryNew::new()` constructor in diesel_models crate
- **Transformation Tracking**: 
  - `transformation_id` field added to staging_entries table (nullable)
  - Links staging entries back to their originating transformation for audit trails
  - Automatically populated during transformation execution
  - Optional in `StagingEntryCreateRequest` for manual API calls (defaults to None)
  - **Pattern**: All transformation functions pass `transformation_id` to `StagingEntryNew::new()`
- **Constructor Signature**: `StagingEntryNew::new()` takes 10 parameters including `transformation_id: Option<String>` as the last parameter
- **API Integration**: `StagingEntryCreateRequest` includes optional `transformation_id` field that gets passed through to the constructor
<!-- AI-UPDATED: 2025-08-13 - Entries metadata encryption in transaction creation -->
- **Entry Metadata Encryption**: Transaction creation now requires encryption context for entry metadata
  - All `TransactionEntry` creation functions made async to support metadata encryption
  - Utils layer functions like `get_new_entries()` require merchant_id and encryptor parameters
  - Core layer responsible for getting merchant encryptor and passing to utils functions
  - Entry creation via `EntryNew::new()` includes encrypted metadata parameter from encryptor
- **Entry Creation with Encryption**: Both Direct and Expected entries encrypt metadata during transaction creation
  - `create_entry()` method in `TransactionEntryExt` handles async metadata encryption
  - Uses `encryptor.encrypt_optional_json_value()` for consistent encryption approach
  - Encryption context propagated from core transaction creation through utils layer
<!-- AI-UPDATED: 2025-01-21 - Update API restrictions and extensibility patterns -->
- **Update API Restrictions**: Transaction/staging entry update endpoints currently only support void status transitions
  - PUT endpoints designed for extensibility but currently restricted to void operations only
  - Pattern: Check current status, validate transition is allowed, then perform atomic update
  - Security: All updates validate profile ownership using logical transaction_id (not database primary keys)
  - Future: Update validation can be expanded to support other status transitions as needed
<!-- AI-UPDATED: 2025-01-21 - Atomic void operation patterns -->
- **Atomic Void Operations**: Transaction void automatically voids related staging entries in same database transaction
  - Pattern: Update transaction status first, then find and void related staging entries by version
  - Consistency: All operations succeed or all rollback - prevents partial void states
  - Data integrity: Uses same version matching to ensure related entities are voided together
  - Error handling: Database transaction rollback on any failure preserves consistency
<!-- AI-UPDATED: 2025-01-21 - Versioned data structure approach for context storage -->
- **Versioned Data Structures**: Both StagingEntryData and TransactionData use V1 versioning pattern
  - Pattern: Enum with version variants (currently only V1) stored in JSONB data column
  - Purpose: Store structured context like error details, void reasons, processing metadata
  - Backward compatibility: New versions can be added without breaking existing data
  - Usage: Optional context storage that doesn't affect normal operation flow
  - Serialization: Direct serde JSON serialization to JSONB column
<!-- AI-UPDATED: 2025-10-30 - Staging Entry V2 with metadata schema validation -->
- **Staging Entry V2 Implementation**: New endpoint for schema-validated staging entry creation
  - **API Endpoint**: `POST /staging_entries/v2` with `StagingEntryCreateRequestV2` request body
  - **Metadata Schema Integration**: Validates raw CSV data against predefined metadata schemas
  - **Validation Pipeline**: Raw Data → `ParsedRow::from_api_fields` → `RowMapper::map_row` → `MappedRow` → `RowValidator::validate_row` → `RowValidationOutcome::Validated(ValidatedEntryData)`
  - **Request Structure**: `{ account_id, metadata_schema_id, raw_data: HashMap<String, Option<String>>, transformation_id, transformation_history_id }`
  - **Schema Validation**: Ensures metadata_schema_id exists and belongs to authenticated user's profile
  - **Data Transformation**: Raw field data mapped and validated against schema before staging entry creation
  - **Error Handling**: New error types `MetadataSchemaNotFound` (404) and `MetadataSchemaValidationFailed` (400)
  - **Encryption Integration**: Validated metadata encrypted using merchant-specific encryptor before storage
  - **Backward Compatibility**: V1 endpoint (`POST /staging_entries`) remains unchanged for existing workflows
<!-- AI-UPDATED: 2026-06-29 - V2 cursor-paginated list for staging entries (and transactions) -->
- **V2 List = cursor pagination, not offset**: `POST /staging_entries/v2/list` (body `ListStagingEntriesV2Request { filters, cursor_payload }`, response `StagingEntryListV2Page = CursorPage { items, next_cursor, prev_cursor }`). The transaction v2 list (`POST /transactions/v2/list` → `TransactionListV2Page`) uses the same shared `common_types::pagination` machinery — see `patterns/DATABASE-QUERY.md` for the keyset mechanics and the `+1`/`has_more`/`Previous`-reversal gotchas.
  - **POST, not GET**: filters + cursor travel in the request body, so the list endpoint is a POST even though it only reads. Permission: `ProfileReconStagingEntryRead`.
  - **Page size clamp**: `STAGING_ENTRY_LIST_DEFAULT_LIMIT = 10`, `STAGING_ENTRY_LIST_MAX_LIMIT = 50` (core layer; `into_query` clamps client `limit` to `[1, 50]`). Never returns unbounded.
  - **`Archived` is never listable — rejected in TWO places**: (1) `ApiStagingEntryListFiltersV2::validate()` returns `UnsupportedStatusFilter` (400) when `status` *or* `detailed_status` explicitly includes `Archived`; (2) the query *unconditionally* adds `status != Archived` even when no status filter is given. Don't relax one without knowing the other exists.
<!-- AI-UPDATED: 2026-08-02 - Flattened staging entry status, step 1 of a 2-step field swap -->
  - **`status` and `detailed_status` are BOTH live filters, on purpose — do not "clean up" the old one.** The DB column `staging_entry.status` is now granular (`needs_manual_review_duplicate_entry`, `needs_manual_review_no_rules_found`, …, `VARCHAR(64)`, `common_enums::DieselStagingEntryStatus`), mirroring what `DieselTransactionStatus` did for transactions. **Only `NeedsManualReview` is granular** — the ten reasons are what an exception queue actually gets filtered by. `Pending` stays one flat status (`pending`) even though `data` distinguishes new from edited: nobody filters "pending, but edited", so splitting it would only add DB values and match arms. If that changes, the split goes exactly where `NeedsManualReview`'s did. Because FE and BE deploy on different cycles, this landed as **step 1 of a 2-step swap**: every pre-existing API field kept its name, type and wire shape, and the granular value is exposed through new `detailed_status` fields alongside them. Step 2 (after FE migrates) drops the coarse fields and renames `detailed_status` → `status`. Until then, deleting the coarse `status` filter, `GetStagingEntryResponse.status`, or `status_breakdown` silently breaks the frontend.
  - **Three status types, same as transactions**: `DomainStagingEntryStatus` (`common_types/src/staging_entry/status.rs`) is hierarchical (`NeedsManualReview(StagingEntryManualReviewData)`, everything else bare) and is what `DomainStagingEntry` carries; `DieselStagingEntryStatus` is the flat storage enum and carries **no** behaviour — flat because it is what the indexed `VARCHAR(64)` column stores; `ApiStagingEntryStatus` is hierarchical too and shares the *same* sub-status types as the domain enum, so the conversion is a five-arm passthrough. It lives in `recon/src/utils/staging_entry.rs` as `DomainStagingEntryStatusExt::to_api_response()`, **not** in `api_models` — cross-layer conversions belong in the recon crate, and the `*Ext` trait (rather than `ForeignFrom`) matches how the rest of that file converts staging entry data. Conversions are total in both directions between domain and diesel, so `DomainStagingEntryStatus::from(diesel_status)` is always safe.
    - **Only `Api <-> Domain <-> Diesel` conversions may exist — never `Api <-> Diesel` directly.** A caller holding a diesel status that needs the API one does the two hops in place (`DomainStagingEntryStatus::from(row.status).to_api_response()`, see `core::overview` and `utils::overview`). Don't reintroduce a shortcut impl. Anything reading a raw diesel row (`StagingEntryOverviewRow`, `StagingEntryAccountView`, the void/link paths) converts up to domain before asking a question of it.
  - **`ApiStagingEntryCoarseStatus` is the compat-only shim** (`api_models`, flat, 5 values). It exists solely so `status` / `discarded_status` / `status_breakdown` keep serializing as the bare strings the frontend reads today — the hierarchical `ApiStagingEntryStatus` cannot, since an adjacently-tagged enum is an object, not a string. It also types the v1 `?status=` query param, which needs `FromStr` and so can only ever be coarse. `ApiStagingEntryStatus::coarse()` maps down to it, `ApiStagingEntryCoarseStatus::to_detailed()` maps back up (to the `None` sub-status, for the one variant that has one). Both die in step 2. Predicates that used to go through a coarse enum now match `DomainStagingEntryStatus` directly and exhaustively — see `recon_engine::process_staging_entry` and `utils::audit_trail::get_staging_entry_events`; `is_voidable()` lives there for the same reason.
  - **`ApiStagingEntryStatusExt::to_domain_statuses()` is the single fan-out**, and it lives in `recon/src/utils/staging_entry.rs` next to its mirror `DomainStagingEntryStatusExt::to_api_response()` (extension traits, since `api_models` owns the type but must not hold the cross-layer conversion). It is what keeps `?status=needs_manual_review` (v1) and `filters.status` (v2) matching the same rows as before, and internal callers that want "every manual-review reason" (`audit_trail`, `exception_management`) call it too rather than hand-listing children. Callers that only want `Pending` — the batch processor in `recon_engine` — name `DieselStagingEntryStatus::Pending` directly, since a non-granular status has nothing to fan out. The `NeedsManualReview(None)` arm iterates `StagingEntryManualReviewData` via `EnumIter`, so a new reason is picked up automatically and there is exactly one place that names the children; every other arm is a one-element `vec!` because only that status is granular. It stops at **domain** on purpose (see the `Api <-> Domain <-> Diesel` rule above); every caller that needs storage values maps the result through `DieselStagingEntryStatus::from` at the call site — `to_diesel_statuses` / `coarse_to_diesel_statuses` in `recon/src/utils/staging_entry.rs` do exactly that for the two filter structs. **`diesel_models` no longer knows about any API enum**: both list queries receive `Vec<DieselStagingEntryStatus>` already fanned out and just `eq_any` it.
  - **`ApiStagingEntryStatus` is adjacently tagged on `status`/`sub_status`**, byte-identical in shape to `ApiTransactionStatus`. The enum is therefore already in its final form; only the *field* carries migration scaffolding. **`GetStagingEntryResponse.detailed_status` is deliberately NOT `#[serde(flatten)]`ed** — the tag key `status` is the one the struct already uses for the coarse value, so flattening would collide. Nested, it lives in its own object scope (`"detailed_status": {"status": …, "sub_status": …}`) and nothing overlaps. Step 2 = delete the coarse `status` field, rename this one to `status`, add the `flatten` — the enum itself never changes. `StagingEntryDetailedStatusAmount` *does* flatten it (no sibling `status` key there), so the overview breakdown needs no step-2 change at all.
  - **The `Option` sub-status is load-bearing FE compatibility, not laziness**: `NeedsManualReview(Option<..>)` lets a caller send a bare `{"status": "needs_manual_review"}` with the `sub_status` key *absent* and still match every reason. This works because serde deserializes a missing adjacently-tagged content field for a newtype variant via `__private::de::missing_field`, whose deserializer answers `deserialize_option` with `visit_none`. `test_list_staging_entries_v2_filters_by_detailed_status` sends that exact raw JSON — keep it, it is the only guard on this behaviour. Note the *serialize* direction always emits `"sub_status": null` for `None`, and responses always build `Some(..)`.
  - **The two filter fields exist only in `api_models`.** `ForeignFrom<ApiStagingEntryListFiltersV2>` resolves `status` and `detailed_status` into the *single* `StagingEntryListFiltersV2::status: Option<Vec<DieselStagingEntryStatus>>` — both fan out to granular, and when both are present the result is their **union**, de-duplicated through a `HashSet` (sending both ORs, it does not narrow). Below that boundary there is one status filter and one `eq_any`. Don't re-add a parallel `detailed_status` field to the common_types/diesel structs.
  - **v1 has the same api/internal split as v2**: `ApiListStagingEntriesFilters` (`api_models`, the `web::Query` extractor and the bulk-operations `Filters` selection) → `ForeignFrom` → `ListStagingEntriesFilters` (`common_types`, plain struct, `Vec<DieselStagingEntryStatus>`). The internal struct has no serde/utoipa derives on purpose — it never touches the wire. Callers inside the service (`audit_trail`, `exception_management`, `bulk_operations`) build the internal one directly.
  - **Status is derived from `data`, never set by hand**: `From<&StagingEntryData> for DomainStagingEntryStatus` (`common_types/src/staging_entry/data.rs`) is the single choke point — it produces the *domain* status, exactly like `TransactionUpdateInternal`. A convenience `From<&StagingEntryData> for DieselStagingEntryStatus` routes through it, so diesel write sites can stay `status: (&data).into()`. `StagingEntryNew::new_versioned()` calls it for *all* update kinds. Adding a new `ProcessingErrorTypeV1` variant therefore means adding a `StagingEntryManualReviewData` variant (which `ApiStagingEntryStatus` shares) + a `DieselStagingEntryStatus` variant; the exhaustive matches will point at every site.
  - **Sort fields**: `StagingEntryListSortField::{ Id, EffectiveAt }`. `EffectiveAt` carries a composite `(effective_at, id)` cursor — the `id` is the mandatory tie-breaker; dropping it would skip/duplicate equal-timestamp rows at page boundaries.
<!-- AI-UPDATED: 2026-07-10 - Fixed cursor precision truncation bug -->
  - **Cursor `effective_at` must serialize at microsecond precision**: `EffectiveAtSortField`/`StagingEntryEffectiveAtSortField` use `custom_serde::iso8601_micros`, not the default millisecond `iso8601`. Postgres `Timestamp` columns store microseconds; the query builder compares the decoded cursor back against the column with `.eq()`/`.gt()`/`.lt()` (`diesel_models::query::transaction`/`staging_entry`). Millisecond truncation broke the `.eq()` tie-break branch, causing rows to skip/duplicate on page navigation. A second, deeper bug in the same area: `iso8601_micros` itself (and plain `iso8601`) misformatted ~20% of subsecond values by 1 unit due to float rounding in `time`'s well-known `Iso8601` formatter — see `patterns/CUSTOM-SERDE.md` gotchas for the fix (explicit `[subsecond digits:N]` format description).
  - **`order_id` filter maps to the `entry_identifier` column** (`dsl::entry_identifier.eq(order_id)`) — there is no `order_id` column.
  - **Shared response builder**: both V1 and V2 list paths funnel through `staging_entry_views_to_response()`, which batch-resolves `modified_by` users in one call. Keep new list variants on this helper instead of re-resolving users inline.
<!-- AI-UPDATED: 2026-08-19 - Per-transaction entry cap in the v2 transaction list -->
  - **v2 list caps entries per transaction at 5** (`TXN_LIST_MAX_ENTRIES_PER_TRANSACTION`, `core/transaction.rs`); `TransactionWithEntriesResponse.has_more_entries` signals the rest, fetched via `GET /transactions/{id}/entries`. Paginating transactions alone bounds nothing — a one-to-many transaction can hold thousands of entries.
    - **No transaction→entry join.** `list_v2_paginated` returns only `transaction ⋈ recon_rule`; `hydrate_transaction_rule_views` (utils) takes the first `cap + 1` ids from each row's `entry_ids`, then batch-fetches via the existing `list_entries_by_ids` and `list_accounts_by_profile_id_and_account_ids` — **one call each for the whole page, never per transaction.** Joining instead fans the transaction row (with its `entry_ids` array) out per entry: measured 25 MB / 14 ms vs 4 MB / 1 ms for a 50×5000 page.
    - **Take `entry_ids` unsorted.** It is creation order with appended entries at the end, so the leading slice is what the transaction was opened with and is stable across calls. Entry ids are random nanoids, so sorting by id would order by nothing.
    - **`has_more_entries` comes from the `cap + 1`-th entry actually returned, not `entry_ids.len()`**, so a stale array can't inflate it. Both lookups in the hydrator (`entries_by_id`, `accounts_by_id`) fail with `InternalServerError` on a miss — they were fetched for exactly those ids, so absence is corruption, not a valid empty case. Don't soften to `if let` / `unwrap_or_default`.
    - **Linking skips `EntryStatus::Expected` — in the scan AND the DB fallback, kept in sync.** A placeholder's metadata is projected from the *source* and its `order_id` is the source's, so it never holds a next-leg value; in one-to-many it coexists with real entries on the same account, so the scan could hit it first. The fallback (`find_non_expected_by_transaction_id_version_and_account`) exists because the cap can hide the real entry. **It is deliberately unordered** (`LIMIT 1`, no `ORDER BY`): with several real entries on the link account any of them links identically, and an `ORDER BY id` made the planner walk `entries_pkey` instead of the transaction index — measured 43 ms vs 0.25 ms on a 5000-entry transaction — which would have needed a dedicated 3-column index. Guarded by `test_linked_transaction_skips_expected_placeholder_entry` (fails without the status filter).
    - **Known limitation:** `order_id`/`metadata` filters match on *any* entry, but the 5 returned are the first-created — a filtered result can show a transaction whose visible entries don't contain the match. Real fix is the entries endpoint.
<!-- AI-UPDATED: 2026-08-20 - Entries list pins a mandatory primary_transaction_id -->
- **Entries List API** (`POST /entries/list`): cursor-paginated list of ONE transaction version's *entries*, built on the same `common_types::pagination` machinery as the staging-entry and transaction v2 lists. **No `/v2` in the path** — there was never a v1 entries list, so the types drop the `V2` suffix too (`ApiEntryListFilters`, `ListEntriesRequest`, `EntryListPage`). Permission: `ProfileReconTransactionRead`. Limits: default 10, max 50 (`domain/entry.rs`).
  - **`primary_transaction_id` is MANDATORY and is the profile scoping.** It sits on `ApiEntryListFilters` as a required (non-`Option`, no serde default) field — reviewer-chosen placement — and is resolved in core with `find_by_id_and_profile` (profile-scoped, 404 → `RE_505`) into the `(logical_transaction_id, transaction_version)` pin that `EntryListFilters` carries as required fields. The query (`diesel_models::query::entry::list_paginated`) is a bare single-table query with **no `account` join and no profile filter** — safe only because the pin comes from that profile-scoped lookup. Don't relax the pin to optional without reinstating profile scoping in the query itself (`entry` has no `profile_id` column; scoping would need `INNER JOIN account` on `account.profile_id`).
  - **No implicit status filtering — archived entries are returned when their version is pinned.** A superseded version's entries are `Archived` (`discard_transaction_with_entries_and_expectations` archives rather than deletes), so pinning an **old** version's primary id returns its archived rows by default — exactly what a version-history view wants. The earlier `status != Archived` default existed to stop a cross-version list interleaving every version's rows; the mandatory version pin now does that job, so don't reintroduce the exclusion (it would make old versions list as empty). `status` remains an ordinary filter (`["archived"]` included) applied within the pinned version.
  - **Entries created through bare `POST /entries` are NOT reachable via this API** — they have no `transaction` row, so their client-supplied transaction id never resolves (404). That is accepted: the list is a per-transaction-version view, not a global entry search.
  - **`GET /transactions/{id}/entries` is deliberately NOT built on this** — it keys on the *logical* id (`list_entries_by_transaction_id_and_version`, unpaginated, unordered, unscoped) while the cursor list keys on a version's *primary* id. Delegating it was tried and reverted: it would have silently changed profile scoping, version pinning, and a 50-row cap for existing callers. Its `// TODO: Validate that all accounts...` therefore still stands — any authenticated caller can read any transaction's entries by id. When that is fixed, the cheap fix is a transaction-level check (`find_latest_transaction_by_logical_id_and_profile_id`, 404 on miss), **not** an entry→account join: `transaction` has its own `profile_id`, so no join is needed and the existing `(logical_transaction_id, transaction_version)` index covers it.
  - There is deliberately **no `ForeignFrom<ApiEntryListFilters>`** — building `EntryListFilters` needs the async, fallible transaction lookup, so core constructs it by hand.
  - **`entry_type` is a single `Option<BalanceDirection>`, not a `Vec`** — reviewer-chosen: `[credit, debit]` is equivalent to no filter, so a list adds nothing. This deviates from the older list endpoints (`ListEntriesFilters`, `ListStagingEntriesFilters`, `TransactionAccountFilters`), which take comma-separated lists.
  - **Other filter names follow the response vocabulary, not the columns**: `entry_type`→`balance_direction`, `order_ids`→`entry_identifier`, `staging_entry_ids`→`primary_staging_entry_id`, `transformation_config_ids`→`transformation_config_id`. The rename happens only in the query builder, exactly as the staging-entry filters do it.
  - **Route order in `app.rs` is load-bearing**: `/entries/list` MUST be registered before `/entries/{entry_id}`. Both are single-segment patterns and actix matches resources in registration order by path *before* method — reversed, a POST to `/entries/list` matches `/{entry_id}`, finds no POST route, and 405s with no compile error. `test_list_entries_route_is_not_shadowed_by_entry_id` is the only guard.
  - **Sort fields**: `EntryListSortField::{ Id, EffectiveAt }`, with the composite `(effective_at, id)` cursor and the mandatory `iso8601_micros` precision — see the staging-entry notes above, the same two gotchas apply verbatim.
  - **Shared response builder**: the entry list paths funnel through `core::entry::entries_to_response()`. Keep new variants on it rather than re-inlining the decrypt-and-map closure (it was duplicated three times before this change).
<!-- AI-UPDATED: 2025-09-08 - Transaction effective_at implementation -->
- **Transaction Effective At**: Transactions include `effective_at` field for business logic filtering
  - **Calculation**: Set to minimum (earliest) `effective_at` from all entries in the transaction
  - **Purpose**: Enables time-based filtering on business effective dates rather than creation timestamps
  - **Time Range Filter**: The `time_range` query parameter filters on transaction `effective_at`, not `created_at`
  - **Database Index**: Indexed for efficient filtering with composite index on (profile_id, effective_at)
  - **API Response**: Included in both `GetTransactionResponse` and `TransactionWithEntriesResponse`
<!-- AI-UPDATED: 2026-07-20 - Documented ManySingle source-only versioning exception -->
- **ManySingle Expected Append Exception**: While an existing `OneToOneManySingleV1` transaction remains `V3::Expected` (no target), additional transaction-mode sources are appended to the same primary transaction id/version. Existing entries do not change classification, so only the new Pending source, the residual Expected entry, and transaction aggregates change. Any target arrival or non-Expected state returns to normal copy-and-replace transaction versioning. See reconciliation AGENT-NOTES for the eligibility and locking contract.

<!-- AI-UPDATED: 2026-09-02 - Entry count cap on recon-driven versioning -->
<!-- AI-UPDATED: 2026-09-28 - Cap is 25k; const moved to domain/transaction.rs and shared with manual recon v2 -->
- **Entry Count Cap**: Recon-driven versioning refuses to build a transaction holding more than `MAX_ENTRIES_PER_TRANSACTION` (25 000, `recon/src/domain/transaction.rs`) entries, `Expected` placeholder included, because each version re-inserts the whole entry set. See reconciliation AGENT-NOTES for where it is enforced and why a confirmation at the cap still passes. Manual reconciliation v2 checks the same constant before loading anything (`MANUAL-RECONCILIATION.md`); v1 does not check it.

<!-- AI-UPDATED: 2025-01-22 - Transaction versioning and void functionality -->
- **Transaction Versioning System**: Transactions implement full versioning pattern similar to hyperswitch core
  - **Primary Key (`id`)**: Unique identifier for each version (e.g., `txn_abc123`, `txn_def456`)
  - **Logical ID (`transaction_id`)**: Groups all versions of same transaction (e.g., `txn_main_xyz789`)
  - **Entry IDs (`entry_id`)**: Array changes with each version - references NEW entry IDs created for that version
  - **Version Field**: Increments with each update (1 → 2 → 3)
  - **Archive Pattern**: Old versions marked as `Archived` status, new versions created with incremented version
  - **Query Pattern**: `list_by_transaction_id()` ordered by `version DESC` to get latest version first
  - **Audit Trail**: Complete history preserved through versioned records
- **Void Transaction Implementation**: Complete atomic operation that maintains data consistency
  - **API Endpoint**: `POST /transactions/{transaction_id}/void` with `VoidTransactionRequest` body
  - **Authorization**: Profile-scoped access, returns 404 for unauthorized/non-existent transactions
  - **Archive Operations**: Sets original transaction to `Archived` status, preserves original data
  - **Search Expectation Cleanup**: Archives search expectations for Expected entries to prevent orphaned reconciliation matches
  - **Version Creation**: Creates new transaction with `Void` status and incremented version
  - **Entry Versioning**: Creates void entries for each original entry with opposite balancing effect
  - **Atomic Consistency**: All operations (transaction, entries, search expectations) in single database transaction
- **Void Transaction Data Model**: 
  - **Request**: `VoidTransactionRequest { reason: String }` for audit purposes
  - **Response**: Complete `GetTransactionResponse` with void transaction details
  - **Transaction Data**: Stores void reason in `TransactionData::V1(TransactionDataV1::Void(VoidData::V1 { reason }))`
  - **Entry Status**: New void entries have `EntryStatus::Void` status
  - **Search Expectations**: Expected entries' search expectations archived with `ExpectationStatus::Archived`
<!-- AI-UPDATED: 2025-09-08 - TransactionVersionUpdate structure changes -->
- **TransactionVersionUpdate Structure**: Enhanced struct for transaction versioning operations
  - **Fields**: `new_entries: Vec<Entry>` (complete Entry objects) and `data: TransactionData` (transaction data)
  - **Purpose**: Provides complete entry objects that the versioned transaction should reference
  - **Automatic Calculations**: Constructor automatically calculates `credit_amount` and `debit_amount` from provided entries
  - **Entry ID Extraction**: Entry IDs automatically extracted from `new_entries` using `.map(|e| e.entry_id.clone()).collect()`
  - **Balance Calculation**: Iterates through entries, summing amounts by `BalanceDirection` (Credit/Debit)
  - **Status Derivation**: Transaction status automatically derived from `data` field using `Into` trait
  - **Constructor Location**: `new_versioned()` in diesel_models with full entry processing logic
  - **No Result Type**: Constructor simplified to return `Self` directly instead of `Result<Self, _>`
  - **Usage Pattern**: Caller creates new Entry objects, then passes complete entries to `TransactionVersionUpdate`
<!-- AI-UPDATED: 2025-01-22 - Transaction resolution endpoint uses ResolutionProvider trait -->
<!-- AI-UPDATED: 2025-10-01 - Direct transaction entity resolution -->
- **Transaction Resolution Endpoint**: `/exception_management/transactions/{id}/resolutions` endpoint accesses transaction data via `ResolutionProvider` trait for exception handling
- **Direct Entity Resolution**: `ResolutionProvider` now implemented directly on `Transaction` entity rather than `TransactionData`
  - **Simplified API**: Call `transaction.get_available_resolutions()` directly instead of `transaction.data.get_available_resolutions()`
  - **Force Reconcile Availability**: `ForceReconcile` resolution offered for `DataMismatch`/`SplitMismatch` status (no ledger check in the resolution mapping — the balance gate lives in `force_reconcile_transaction` at execution time)
  - **Status-Based Logic**: Resolution availability determined by `transaction.transaction_status` field
  - **Performance**: No need to access nested data field for resolution determination
<!-- AI-UPDATED: 2026-07-01 - Resolution mapping: ForceReconcile no longer ledger-gated; CurrencyMismatch now voidable/replaceable -->
- **Force-reconcile is gated in two places (keep the status set in sync)**: `force_reconcile_transaction` (core) accepts `DataMismatch | SplitMismatch` **and** requires a balanced ledger (`validate_ledger_integrity()`); `get_available_resolutions` offers `ForceReconcile` for the same statuses but **without** a ledger check. For `DataMismatch` this never diverges (amounts always balance); for an unbalanced `SplitMismatch` the endpoint lists `ForceReconcile` even though the backend rejects it as `UnbalancedTransaction`.
- **`CurrencyMismatch` resolutions**: `[ReplaceEntries, VoidTransaction]` — deliberately no `ForceReconcile` (currency differences can't be ignored) and no `CreateEntries`/`Link` (can't add entries under a mismatched currency). The only true `[]` dead-ends are `Posted(Manual)`, `Archived`, and `Void`.
- **`Expected` gained `CreateEntries`**: `Expected` and the amount-mismatch statuses now all share `[CreateEntries, LinkStagingEntriesToTransaction, ReplaceEntries, VoidTransaction]`.
- **Resolution test helper quirk**: `create_test_transaction` outcome is driven by `data_mismatch_type`, NOT the `transaction_status` label — `DataMismatchType::Metadata`/`Currency` both yield balanced `DataMismatch`. So the `resolutions` case labeled `UnderAmount(Expected)` actually exercises `DataMismatch`; `CurrencyMismatch`/`SplitMismatch` resolutions are unreachable via this helper (currently untested).

<!-- AI-UPDATED: 2025-09-21 - Transaction Resolution Implementation -->
## Transaction Resolution Gotchas
- **Expected Entry Conversion**: `create_combined_transaction_entries()` must preserve Expected entries and their search expectations
- **Validation Outside Transaction**: Entry combination and balance validation done BEFORE database transaction to avoid generic `DatabaseError::Others` mapping - encryption errors should be caught as proper `TransactionError` types
- **Recon Engine Priority**: One-to-many resolution (100 credit + 80 debit + 20 expected) stays `Expected` status even when balanced - presence of Expected entries takes priority over balance in status determination
<!-- AI-UPDATED: 2025-09-23 - Force reconcile transaction implementation -->
<!-- AI-UPDATED: 2025-10-01 - Resolution mapping refactor -->
- **Force Reconcile Transaction**: Manual resolution of mismatched transactions with balanced amounts
  - **API Endpoint**: `PUT /exception_management/transactions/{transaction_id}/force_reconcile` with `ForceReconcileRequest` body
  - **Purpose**: Allows operators to resolve mismatched transactions when core business rules are satisfied
  - **Validation Requirements**: 
    - Transaction status must be `DataMismatch` or `SplitMismatch` (no other status is accepted — `CurrencyMismatch` is NOT force-reconcilable)
    - Credit amounts must equal debit amounts (balanced transaction required)
    - All amounts must be positive (basic sanity check)
    - Currency validation already handled by transaction creation flow
  - **Business Logic**: Force reconciled transactions convert `Mismatched` → `Posted` status
  - **Versioning Pattern**: Creates new transaction version with incremented version number, archives original
  - **Entry Status Change**: All entries in force reconciled transaction get `EntryStatus::Posted`
  - **Audit Trail**: Data is stored as `TransactionData::V1(TransactionDataV1::Posted)`
  - **Exception Management Integration**: `ForceReconcile` action available for mismatched transactions in resolution mapping
- **Force Reconcile Validation Function**: `validate_ledger_integrity()` in utils/transaction.rs
  - **Status Check**: Only allows force reconcile on `DataMismatch` / `SplitMismatch` status
  - **Balance Validation**: Verifies credit_amount equals debit_amount using stored transaction amounts
  - **Amount Validation**: Ensures both credit and debit amounts are positive (sanity check)
  - **Security**: Currency validation already enforced during transaction creation, no additional checks needed
  - **Synchronous Operation**: Method is synchronous (no async) as it only validates stored amounts
- **Force Reconcile Testing Pattern**: 
  - **Success Test**: Create realistic metadata mismatch scenario using recon engine with balanced amounts
  - **Failure Tests**: Test wrong status (Posted), unbalanced amounts, and transaction not found
  - **Recon Rule Required**: Must create proper recon rule with metadata match requirements to generate valid mismatched transactions
  - **Staging Entry Flow**: Use staging entry → recon engine → mismatch detection → force reconcile pattern for comprehensive testing
  - **Conditional Force Reconcile Testing**: Test that force reconcile is only available for balanced mismatched transactions
    - **Force Reconciliable**: Metadata mismatches with balanced amounts should offer `ForceReconcile` resolution
    - **Non-Force Reconciliable**: Amount mismatches (unbalanced) should NOT offer `ForceReconcile` resolution
    - **Test Pattern**: Use different `MismatchType` values to create different scenarios
<!-- AI-UPDATED: 2025-09-30 - Transaction data versioning with posted variants -->
- **Transaction Data V2 Evolution**: Transaction data structure evolved from simple enum to versioned with detailed posted variants
  - **Version Migration**: TransactionData now has V1 (legacy) and V2 (current) variants stored in JSONB data column
  - **Posted Data Granularity**: V2 Posted status includes detailed `PostedData` enum specifying how transaction was posted:
    - `PostedData::V1(PostedDataV1::Reconciled)` - Normal reconciliation through recon engine
    - `PostedData::V1(PostedDataV1::ForceReconciled(reason))` - Manual force reconcile with audit reason
  - **Backward Compatibility**: V1 transactions preserved in `archived::TransactionDataV1` module
  - **API Response Mapping**: Complex conversion layer translates between diesel models and simplified API response models
  - **Resolution Action Mapping**: Posted variants affect available exception management actions via ResolutionProvider trait
- **Posted Variant Implementation Gotchas**:
  - **Data Structure Nesting**: V2 uses deeply nested tagged enums: `TransactionDataV2::Posted(PostedData::V1(PostedDataV1::ForceReconciled(ForceReconciledData::V1(reason))))`
  - **API Simplification**: API models flatten complex nested structure to simpler `PostedData { Reconciled | ForceReconciled { reason } }` format
  - **Version Tagging**: Each nested level has version tag (`data_version`, `posted_version`, `posted_type`, `force_reconciled_version`) for future extensibility
  - **Conversion Traits**: `TransactionDataExt` trait handles complex conversion from diesel models to API responses with proper nesting resolution
  - **Legacy Support**: Existing V1 transactions continue working through archived module while new transactions use V2 structure
  - **Force Reconcile Audit**: Reason field mandatory for force reconciled transactions to maintain audit trail for manual interventions
- **Implementation Pattern for Posted Variants**:
  - **Creation**: Use `TransactionDataV2::Posted(PostedData::V1(PostedDataV1::Reconciled))` for normal recon engine flow
  - **Force Reconcile**: Use `PostedDataV1::ForceReconciled(ForceReconciledData::V1(ForceReconciledDataV1 { reason }))` for manual resolutions
  - **Resolution Mapping**: Posted transactions have different available actions based on posted type (Reconciled vs ForceReconciled)
  - **Database Storage**: All variants serialize to JSONB with proper version tags for queryability and future migrations
  - **API Conversion**: Always use trait implementations to convert from complex diesel models to simple API responses


<!-- AI-UPDATED: 2025-12-19 - Reconcile Transaction API for Exception Management -->
## Exception Management - Reconcile Transaction API

<!-- AI-UPDATED: 2026-09-22 - v2 delta endpoint added; see MANUAL-RECONCILIATION.md -->
<!-- AI-UPDATED: 2026-09-28 - Corrected v1 request shape, accepted statuses and resulting status against the code -->
> This section describes **v1**. The delta endpoint (`.../manual_reconciliation/v2`), what v1 silently
> loses that v2 fixes, and the metadata/search-key rules both depend on are in `MANUAL-RECONCILIATION.md`.

- **Replace-the-whole-set semantics**: the client sends every entry of the new version; all existing entries are archived and the set is rebuilt from the request
- **API Endpoint**: `POST /exception_management/transactions/{id}/manual_reconciliation` with `ManualReconciliationRequest` body; `{id}` is the version's primary id
- **Request Structure**:
  - `transaction_entries: Vec<TransactionEntryWithData>` - each entry with its own `account_id`, `currency`, `data` state and optional `staging_entry_id` (the staging row's primary `id`)
  - `reason: Option<String>` - stored on the resulting `Matched`/`PartiallyReconciled` data
- **Transaction State Acceptance**: the same nine statuses as v2's `MANUALLY_RECONCILABLE_STATUSES`, hand-listed in a `match`; anything else is 400 `InvalidTransactionStatus`
- **Status Logic** (custom, not the recon engine's):
  - Any `Expected` entry sent → `PartiallyReconciled`; entries keep the states sent
  - Otherwise balanced → `Matched(Manual)`, and every entry becomes `Matched`
  - Otherwise → `PartiallyReconciled`; entries keep the states sent
  - Entries sent as `Matched`/`Posted`/`Void`/`Archived` → 400 `InvalidEntryStatus`
- **No Search Expectations**: Does not create or store search expectations
- **Atomic Operations**: All operations wrapped in database transaction for consistency
- **Staging Entry Processing**: Archives original staging entries and creates processed versions with search duplicates (`create_new_search_duplicate`). Staging entries already `Processed` are skipped, which is how re-sent entries lose their lineage

<!-- AI-UPDATED: 2025-11-08 - Void validation implementation -->
## Void Operation Validation Gotchas

- **Transaction Void Validation**: Added `is_voidable()` method to prevent invalid void operations
  - **Voidable States**: Only `Expected`, `Mismatched`, `PartiallyReconciled`, and `Posted` transactions can be voided
  - **Non-Voidable States**: `Archived` and `Void` transactions cannot be voided (already finalized)
  - **Error Type**: Returns `InvalidTransactionStatus` error with list of expected valid statuses
  - **Validation Location**: Check happens in `void_transaction()` core function AFTER existence check but BEFORE versioning
- **Resolution Mapping Update**: Posted transactions now support void operations in exception management
  - **Posted Resolution Actions**: Posted transactions can now be voided via `VoidTransaction` resolution action
  - **Test Impact**: Updated transaction resolution tests to reflect Posted transactions support voiding
- **Error Context Preservation**: Void validation errors use specific error types instead of generic database errors
  - **HTTP Status Codes**: Validation errors return proper 400 status instead of 500 database errors
  - **User Experience**: Clear error messages indicate why void operation failed and which states are valid

<!-- AI-UPDATED: 2026-09-24 - Entry tenant-scoping and POST /entries removal -->
## Entry Tenant Scoping Gotchas

- **`entry` has no `profile_id` column** - only `merchant_id` (`schema.rs`). The tenant boundary for
  an entry runs through the **account** it posts to, since an account belongs to exactly one profile
  (`RULES.md:39`). Scope entry reads with `find_account_by_profile_id_and_account_id` (single) or
  `list_accounts_by_profile_id_and_account_ids` (batched), as `get_entry_by_id` and
  `list_entries_by_transaction_id` do.
- **Do NOT gate entry reads on the `transaction` table.** Manual entries historically produced no
  `transaction` row, so a transaction-level guard 404s the owner on their own data. This was
  attempted and broke `test_get_entries_by_transaction_id_{with_entries,empty}`.
- **Two transaction id spaces.** `transaction.id` is the physical per-version PK
  (`find_by_id_and_profile`, used by `POST /entries/list`); `transaction.transaction_id` is the
  logical id stable across versions (`find_latest_transaction_by_logical_id_and_profile_id`, and what
  `GET /transactions/{transaction_id}/entries` takes). Entries join on the logical id
  (`entry.logical_transaction_id`). Mixing them 404s every legitimate request.
- **Filter before decryption.** `entries_to_response` uses an encryptor keyed on `merchant_id`, so
  foreign ciphertext reaching it decrypts with the wrong key (500 under `key_manager` mode). The
  account filter must run *before* `entries_to_response`, not after.
