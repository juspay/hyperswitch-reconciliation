# Reconciliation System - Agent Notes

## Critical Rules

- **Staging to Entry Flow**: Staging entries MUST be validated before becoming final entries
- **Status Progression**: Entries follow strict status transitions:
  - Staging: `Pending` → (validate) → Create Entry
  - Entry: `Pending` → `Expected` → `Posted` (or `Mismatched`)
- **No Direct Entry Creation**: Entries should typically come from staging entries, not created directly
- **Soft Deletes Only**: Entries use `discarded_at` and `discarded_status` - never hard delete
- **Balance Reversal on Archive**: When entries are archived, account balances are automatically reversed via `update_account_balance_to_archived`
- **Amount Validation**: All amounts must be positive - use BalanceDirection for debit/credit
- **Effective Timestamp Required**: Both staging entries and entries MUST have an effective_at (TIMESTAMP NOT NULL)
- **Processing Mode Required**: All staging entries MUST specify processing_mode (Transaction or Confirmation)
- **Strategy-Based Execution**: All reconciliation logic driven by `ReconRule.strategy` JSONB column - single source of truth
- **Account→Rule Mapping**: Use `recon_rule_account` table for efficient rule discovery by account type (source/target)
<!-- AI-UPDATED: 2025-07-16 - Version field requirement for entries -->
- **Entry Version Field Required**: All entries now require `version: i32` field (default 1) for optimistic locking
<!-- AI-UPDATED: 2025-07-22 - Transformation tracking in staging entries -->
- **Transformation Tracking**: Staging entries now track their transformation origin
  - `transformation_id`: NOT NULL - identifies the transformation config that created the entry
  - `transformation_history_id`: NULLABLE - identifies the specific transformation execution instance
  - Direct API-created entries have NULL transformation_history_id
  - Used for audit trails and linking staging entries back to their data source
<!-- AI-UPDATED: 2025-08-13 - Staging entries metadata encryption -->
- **Metadata Encryption**: All staging entry metadata encrypted using merchant-specific keys
  - Database schema: `metadata` column changed from JSONB to BYTEA for encrypted storage
  - `merchant_id` column added for encryption key isolation and security
  - Uses `Identifier::ReconMerchant(merchant_id)` for per-merchant key derivation
  - Batch decryption implemented in `list_staging_entries_by_profile_id` for performance
<!-- AI-UPDATED: 2025-08-13 - Entries metadata encryption implementation -->
- **Entries Metadata Encryption**: Entries table metadata encryption following exact staging_entries pattern
  - Database schema: `metadata` column changed from JSONB to BYTEA, `merchant_id` column added
  - 3-layer architecture: API (simple) → Diesel (encrypted) → Domain (decrypted for business logic)
- **Metadata Required**: Both staging entries and entries metadata fields are now mandatory (NOT NULL)
  - Database schema: metadata columns converted from Nullable<Bytea> to Bytea
  - API: metadata field must be provided in all create/update requests
  - Empty metadata stored as empty bytea (\x) when no data provided
<!-- AI-UPDATED: 2025-09-04 - Staging entry versioning implementation -->
- **Staging Entry Versioning**: Comprehensive versioning system following transaction versioning pattern
  - Primary key changed from `staging_entry_id` to new `id` field for proper versioning
  - `staging_entry_id` becomes logical identifier, can have multiple versions with same staging_entry_id
  - Version tracking: `version` INT4 field (default 1), incremented for each new version
  - Soft delete pattern: `discarded_at`, `discarded_status`, `discarded_data` fields for archiving old versions
  - "Copy and replace" strategy: Status changes create new versions instead of updating existing records
  - Schema migration: `2025-09-03-114608_add_versioning_to_staging_entries` implements comprehensive versioning
  - Archive status: `Archived` status marks superseded versions
<!-- AI-UPDATED: 2025-09-29 - Entry data field for contextual status information -->
- **Entry Data Field**: Entries now include JSONB `data` field for contextual status information
  - Schema: `data JSONB NOT NULL` column added via migration `2025-09-24-102442_add_data_field_to_entries`
  - Versioned structure: `EntryData::V1(EntryDataV1)` for future extensibility
  - Status-specific data: `Pending`, `Expected`, `Posted`, `Mismatched(MismatchedData)`, `Archived`, `Void`
  - Detailed mismatch types: `AmountMismatch`, `BalanceDirectionMismatch`, `CurrencyMismatch`, `MetadataMismatch`
<!-- AI-UPDATED: 2025-12-19 - PartiallyReconciled transaction status -->
- **PartiallyReconciled Status**: New transaction status for unbalanced reconciliation attempts
  - Added to `TransactionStatus` enum for partial reconciliation scenarios
  - Used by reconcile transaction API when balance validation fails
  - Available as resolution action type for further reconciliation attempts
  - Enables iterative reconciliation workflow for complex mismatches
  - Variance calculation: Amount mismatches include exact variance between expected and actual amounts
  - All entry creation points: Must provide `EntryData` parameter to `EntryNew::new()`
<!-- AI-UPDATED: 2025-10-03 - Entry data versioning evolution for V2 -->
- **Entry Data V2 Evolution**: Major versioning update to support current and archived entry data patterns
  - New structure: `EntryData::V2(EntryDataV2)` for current entries, `EntryData::V1(archived::EntryDataV1)` for legacy
  - V2 improvements: Simplified enum structure with same variants but cleaner architecture
  - Archive organization: V1 types moved to `common_types::entry::archived` module for historical data
  - Status conversion: Both V1 and V2 support `From<EntryData>` to `EntryStatus` trait for unified status handling
  - Migration path: New entries use V2, existing V1 entries remain functional via archived module
<!-- AI-UPDATED: 2025-10-03 - Entry discarded_data field for versioning consistency -->
- **Entry Discarded Data Field**: Added `discarded_data` column following transaction/staging_entry versioning pattern
  - Database schema: `discarded_data JSONB` (nullable) column added alongside existing `data` field
  - Purpose: Preserves original entry data when entries are archived/discarded for audit trails
  - Update pattern: `EntryUpdate::DiscardUpdate` enum replaces previous `DiscardEntry` struct for cleaner architecture
  - API exposure: All entry endpoints now include `discarded_data: Option<EntryData>` field in responses
  - Consistency: Maintains same versioning approach as transactions and staging_entries tables
- **Search Expectation Versioning**: New `new_versioned()` constructor for proper entry versioning
  - Creates new search expectation linked via `primary_transaction_id` to the new transaction version
  - Preserves all other search expectation fields (rule_id, source/target accounts, search keys)
  - Used during ProcessAsConfirmation when creating new entry versions
<!-- AI-UPDATED: 2026-05-04 - search_expectation FK consolidated to primary_transaction_id -->
  - **Linkage**: A search_expectation row points to its owning transaction via `primary_transaction_id`, which equals `transaction.id` (the per-version primary key) — NOT `transaction.transaction_id` (the logical id shared across versions)
  - **Why this matters**: lookups by logical id will not find expectations; always resolve through the per-version `id`. Older `expectation_id` / `logical_transaction_id` / `transaction_version` columns no longer exist
<!-- AI-UPDATED: 2026-03-12 - modified_by audit column for staging entries -->
- **Modified By Tracking**: Staging entries track who modified them via `modified_by` column
  - Database schema: `modified_by VARCHAR(64)` nullable column with partial index
  - User-initiated changes (API): Populated with `auth_context.user_id` from JWT claims
  - System-initiated changes (recon engine): Set to NULL (no user context)
  - Tracked on: creation, void operations, field updates, status transitions
  - Enables audit trail of who made changes to staging entries
<!-- AI-UPDATED: 2025-10-15 - Staging entry tracking in entries table -->
- **Staging Entry Lineage Tracking**: Entries now track their source staging entries and transformations
  - Database schema: Added `staging_entry_id VARCHAR(64)` and `transformation_id VARCHAR(64)` columns to entries table
  - Both fields NULLABLE - NULL for manually created entries, populated for staging-entry-derived entries
  - Stores primary_key `id` (not logical identifier `staging_entry_id`) of the staging entry
  - **ProcessAsTransaction flow**: Main/direct entry gets staging tracking, expected entries initially NULL
  - **ProcessAsConfirmation flow**: Confirmation populates staging_entry_id when replacing expected entries
  - **Exception management**: Manual replacements use NULL, linking staging entries populates fields
  - **Complete audit trail**: Entry → staging_entry_id → transformation_id → ingestion_history_id → external file
  - **DirectTransactionEntry API**: Added optional `staging_entry_id` and `transformation_id` fields to API model
  - **Field propagation**: Values flow through DirectTransactionEntry → create_entry() → EntryNew → database
  - Indexed for query performance: Partial indexes on both columns (WHERE NOT NULL)

## Common Gotchas

<!-- AI-UPDATED: 2025-06-24 - Processing mode confirmed as required field -->

- **Processing Mode Required**: All staging entries MUST specify processing_mode (Transaction or Confirmation)
- **Currency Mismatch**: Staging entry currency MUST match account currency
- **Profile Authorization**: Always validate account belongs to authenticated profile before creating entries
- **Status Immutability**: Once an entry reaches `Posted` status, it cannot be changed
- **Discarded Entries**: Still visible in queries unless explicitly filtered out
- **FloatMajorUnit Conversion**: API accepts FloatMajorUnit but stores as MinorUnit - beware precision
<!-- AI-UPDATED: 2025-06-27 - Import path reorganization -->
- **Import Path Changes**: common_types now uses module paths (e.g., `common_types::amount::AmountResponse` instead of `common_types::AmountResponse`)
<!-- AI-UPDATED: 2025-06-30 - ProcessAsTransaction implementation gotchas -->
- **Currency Validation Required**: Target accounts MUST have same currency as source staging entry - validated before transaction creation
- **ReconTriggerResponse Pattern**: API now returns Success(transaction) or Failure(reason) instead of just ProcessingDecision enum
<!-- AI-UPDATED: 2026-07-30 - Currency mismatch classification and resolution eligibility -->
- **DataMismatch vs CurrencyMismatch resolution eligibility differs**: `DataMismatch`/`SplitMismatch` grant `ForceReconcile` in `resolution_mapping.rs`; `CurrencyMismatch` doesn't. Misclassifying one as the other silently changes which resolution actions a merchant can take, not just the label shown.
- **Account Batch Validation**: Use list_accounts_by_profile_id_and_account_ids for O(1) validation of multiple accounts
- **Already Processed Check**: Staging entries with status=Processed are rejected immediately with specific error
- **Rule Selection Is Final**: Once highest priority rule selected, no fallback to lower priority rules if processing fails
<!-- AI-UPDATED: 2025-02-07 - ProcessAsConfirmation implementation gotchas -->
- **Multi-Target Search Complexity**: Confirmation mode searches ALL recon rule targets for account, not just first match - can be expensive
- **Transaction Versioning**: Confirmation creates new transaction version (incremented) while archiving old version - maintains same transaction_id
- **Archive vs Soft Delete**: Confirmation uses "Archived" status instead of standard soft delete pattern - different from other entities
- **Expected ID Continuity**: Must map old expected entry IDs to new ones to maintain search expectation chains across versions
- **Match Rule Field Direction**: Match rules compare staging entry target_field vs expected entry source_field - direction matters
- **Status Priority Logic**: Entry status determined by priority: mismatched > expected > posted - not just match result
<!-- AI-UPDATED: 2025-07-03 - Effective timestamp implementation -->
- **Effective Timestamp Type**: Uses `time::PrimitiveDateTime` type for TIMESTAMP columns in database
- **Effective Timestamp Propagation**: Staging entry's effective_at propagates to all created entries during processing
<!-- AI-UPDATED: 2025-08-18 - Duplicate detection implementation gotchas -->
- **Missing Transformation Config**: Not all staging entries have transformation configs (direct API uploads) - duplicate detection gracefully skips these
- **Missing Unique Fields**: When transformation config specifies unique field but staging entry lacks it, use ProcessingError::MissingUniqueField (triggers NeedsManualReview status)
- **StorageErrorExt Implementation**: ProcessingError needed custom StorageErrorExt implementation to use to_duplicate_response pattern for database constraint violations
- **Error Context Specificity**: Hash failures should use ProcessingError::InternalError, not DatabaseError - distinguish internal computation errors from database issues
- **API Timestamp Format**: Effective timestamps in API use ISO format (YYYY-MM-DDTHH:MM:SSZ) with schema annotation `value_type = String`
<!-- AI-UPDATED: 2025-01-07 - Custom serde for PrimitiveDateTime -->
- **Custom ISO8601 Serialization**: All PrimitiveDateTime fields use `common_utils::custom_serde::iso8601` for consistent formatting
- **Millisecond Precision**: Timestamps serialize with 3 decimal digits for seconds (e.g., "2024-01-01T00:00:00.000Z")
- **UTC Normalization**: All timestamps converted to UTC during serialization/deserialization
- **Optional DateTime Handling**: Optional<PrimitiveDateTime> fields use `common_utils::custom_serde::iso8601::option`
<!-- AI-UPDATED: 2025-07-15 - Currency validation comprehensive requirements -->
- **Comprehensive Currency Validation**: Both transaction and confirmation processing validate ALL involved accounts have matching currencies
- **Staging Entry Account Inclusion**: Currency validation MUST include the staging entry's own account, not just target accounts
- **StagingEntryCurrencyMismatch Error**: Currency mismatches return specific error with both staging_currency and account_currency details
- **Multi-Account Currency Check**: Use `list_accounts_by_profile_id_and_account_ids` for batch currency validation across all accounts in operation
<!-- AI-UPDATED: 2025-07-16 - Version field gotchas for entry creation -->
- **Version Field in Entry Creation**: All entry creation functions now require `version: i32` parameter
  - ProcessAsTransaction flow: Passes version=1 to all created entries
  - ProcessAsConfirmation flow: Uses incremented version from evolved transaction
  - Manual entry creation: Must explicitly provide version parameter
<!-- AI-UPDATED: 2026-03-06 - Tolerance feature gotchas from recent reconciliation changes -->
- **Tolerance Strategy Scope**: Tolerance config is supported only for `OneToOneSingleSingle`; do not assume SingleMany/ManySingle/ManyMany support it.
- **Tolerance Validation Rules**: `lower_tolerance` and `upper_tolerance` must be non-negative, and `tolerance_account_id` must exist in the same profile.
- **Inclusive Boundary Behavior**: Amount delta equal to tolerance is treated as within tolerance (`<=`), not as mismatch.
- **Metadata Priority Over Tolerance**: Within-tolerance amount can still end in `DataMismatch` when metadata validation fails.
- **Balancing Entry Side Effect**: Within-tolerance reconciliation creates an additional balancing entry on `tolerance_account_id`.
- **Status Semantics**: Successful tolerance reconciliation maps to `Matched(WithTolerance)` / `MatchedWithTolerance`.
<!-- AI-UPDATED: 2025-08-13 - Entries metadata encryption gotchas -->
- **Domain vs Diesel Model Field Access**: Reconciliation engine must convert Diesel entries to Domain entries for metadata field access
  - ProcessAsConfirmation: Uses `DomainEntry::convert_back()` to decrypt expected entry before field extraction
  - Utils functions: Updated to work with Domain models instead of Diesel models for metadata access
  - Architecture pattern: Core layer handles encryption/decryption, utils layer works with decrypted domain models
- **Hash Trait Required for Reconciliation**: Reconciliation engine uses `HashMap<Entry, EntryNew>` requiring Hash implementation
  - Hash trait implemented for Encryption type by hashing the exposed inner value
  - Critical for reconciliation engine's entry mapping functionality in process_confirmation.rs
- **Async Encryption Context Propagation**: Utils layer functions made async to support encryption operations
  - Transaction creation functions now require merchant_id and encryptor parameters
  - Encryption context must be passed down from core layer through entire call chain
- **Architecture Boundary Enforcement**: Utils layer cannot handle encryption directly - must receive encryption context
  - Core layer responsibility: Get merchant encryptor and pass to utils functions
  - Utils layer responsibility: Use provided encryptor for metadata encryption during entry creation
<!-- AI-UPDATED: 2025-09-04 - Staging entry versioning gotchas -->
- **Staging Entry Versioning Gotchas**: Critical considerations for versioned staging entry operations
  - **Primary Key vs Logical ID**: Always use `id` (primary key) for database operations, `staging_entry_id` for logical grouping
  - **API Response Changes**: All staging entry responses now include both `id` and `version` fields
  - **Test Pattern Changes**: Tests must use `staging_entry.id` for processing endpoints, not `staging_entry_id`
  - **Latest Version Queries**: To get latest version, filter by `staging_entry_id` and `discarded_at IS NULL`, order by `version DESC`
  - **Recon Engine Integration**: All three core functions updated to use versioning (process_transaction_context, process_confirmation_context, process_error_context)
  - **Atomic Versioning**: Each status change archives old version and creates new version in single database transaction
  - **Version Incrementing**: New versions always increment from previous version number (not reset to 1)
  - **Void Operations**: Use existing versioning pattern instead of in-place updates for consistency

## Design Decisions

<!-- AI-UPDATED: 2025-06-24 - Processing mode implementation confirmed -->

- **Why Processing Mode Enum**:
  - Transaction mode: For standard transaction-based entries
  - Confirmation mode: For confirmation-based entries requiring different processing logic
  - Enables different reconciliation workflows based on entry type
  - Required field in staging_entries table
- **Why Staging Entries**:
  - Allows validation before affecting account debit/credit components
  - Provides buffer for external data import
  - Enables bulk validation and error handling
- **Why Soft Deletes**:
  - Maintains audit trail for compliance
  - Allows recovery from mistakes
  - Preserves balance history accuracy
- **Why Status Enum**:
  - Enforces business process flow
  - Prevents invalid state transitions
  - Makes reconciliation state explicit
  <!-- AI-UPDATED: 2025-06-27 - BalanceDirection utility method -->
- **BalanceDirection::opposite()**: Utility method for automatic entry type reversal in double-entry reconciliation
<!-- AI-UPDATED: 2025-06-27 - ProcessingContext and ProcessingError types -->
- **ProcessingContext Enum**: Wraps different processing outcomes (currently only ProcessAsTransaction)
- **ProcessingError Type**: Represents business logic failures (missing fields, no rules) not system errors - intentional design
<!-- AI-UPDATED: 2025-09-04 - Staging entry versioning design decision -->
- **Why Staging Entry Versioning**:
  - **Audit Trail**: Complete history of all staging entry state changes for compliance and debugging
  - **Consistency**: Follows same pattern as transaction versioning for unified approach
  - **Data Integrity**: Prevents data loss during status transitions and provides rollback capabilities
  - **Debugging**: Full visibility into processing flows and decision points
  - **Copy and Replace**: Creates new versions instead of modifying existing records for safer operations
  - **Primary Key Structure**: Two-ID system (primary key `id` vs logical `staging_entry_id`) enables efficient versioning

## Relationships

<!-- AI-UPDATED: 2025-06-24 - Updated rule orchestration relationship -->

- **Accounts**: Staging entries and entries must reference valid accounts
- **Transactions**: Entries are grouped by transaction_id for atomic operations
- **Business Profile**: Indirect through account relationship
- **Rule Orchestration**: Staging entries can be processed by simplified rule orchestration (future integration)

## When Working Here

<!-- AI-UPDATED: 2025-06-24 - Confirmed processing_mode requirements -->

- **Creating Staging Entries**:
  - Validate account exists and belongs to profile
  - Ensure amount is positive
  - Check currency matches account currency
  - MUST specify processing_mode (Transaction or Confirmation)
- **Processing Staging Entries**:
  - Check processing_mode to determine processing path
  - Transaction mode: Standard transaction processing workflow
  - Confirmation mode: Confirmation-based processing workflow
  - Use staging entry status updates for workflow control
  - NeedsManualReview status for entries requiring human intervention
- **Querying Entries**:
  - Filter out discarded entries unless specifically needed
  - Always include status and processing_mode in response
  - Consider pagination for large datasets

## API Behavior

<!-- AI-UPDATED: 2025-06-24 - Confirmed processing_mode in API -->
<!-- AI-UPDATED: 2025-07-03 - Added effective_date requirements -->

- **Staging Entry Creation**:
  - Accepts FloatMajorUnit amounts
  - Auto-generates staging_entry_id with prefix
  - Sets status to Pending automatically
  - Requires processing_mode field (Transaction or Confirmation)
  - Requires effective_date field (ISO date format YYYY-MM-DD)
  - Returns processing_mode and effective_date in all staging entry responses
- **Entry Creation**:
  - Requires transaction_id for grouping
  - Requires effective_date field (ISO date format YYYY-MM-DD)
  - Validates against account currency
  - Updates account debit/credit components based on entry's BalanceDirection
- **Status Updates**:
  - Only specific transitions allowed
  - Posted entries become immutable
  - Discarding preserves original status
  - Status changes update account debit/credit components (pending→posted)
<!-- AI-UPDATED: 2026-05-13 - filter+rename+auth changes; empty list is 200, not 404 -->
- **`GET /search_expectations/primary_transaction/{primary_transaction_id}`**:
  - **Auth + tenant isolation**: JWT-authenticated with `Permission::ProfileReconTransactionRead`. search_expectation rows have no `profile_id` column of their own, so isolation is enforced **indirectly** — the core first calls `Transaction::find_by_id_and_profile(primary_transaction_id, auth.profile_id)` and returns `SearchExpectationError::TransactionNotFound` (RE-1501, 404) if that lookup fails. Only after the owning transaction is verified does it list the expectations.
  - Returns `Vec<SearchExpectationResponse>` for the *exact* `primary_transaction_id` (per-version, not the logical `transaction_id`). Use `transaction.id` from a recent transaction response — querying an old version's id is legal but only surfaces rows linked to that version.
  - Returns **only `status = Expected`** rows — Archived expectations are filtered at the DB layer (`SearchExpectation::list_expected_by_primary_transaction_id`). A transaction with no live expectations responds `200 []`, not 404.
  - **Error variants**: only `TransactionNotFound` (RE-1501, HTTP 404) — transaction doesn't exist OR belongs to another profile (intentionally indistinguishable to prevent profile-id enumeration). There is no `SearchExpectationNotFound` — an empty result is a success, not an error.
  - **Pattern to copy for similar profile-less child resources**: do a transaction-by-profile lookup, `change_context` the error to a parent-not-found variant, then proceed to the child query. Avoids leaking child-resource existence across tenants.

## Recon Rules Core Architecture

- **Strategy Schema**: `ReconRule.strategy` JSONB column stores versioned reconciliation logic
- **Strategy Types**: Currently supports OneToOne variants (SingleSingle, SingleMany, ManySingle)
- **Rule Orchestration**: Automated discovery, filtering, and execution of rules based on staging entries
- **Two Processing Modes**: Transaction (creates expectations) and Confirmation (fulfills expectations)
- **Atomic Operations**: All rule execution uses database transactions for consistency
- **Versioned Compatibility**: All strategy components use versioned enums for backward compatibility

### Strategy Components
- **Triggers**: Field-based conditions that activate rules (equals/not_equals operators)
- **Match Rules**: Field comparisons for validation (amount, currency, metadata)
- **Search Identifiers**: Field mappings for expectation lookup and creation
- **Account Mapping**: `recon_rule_account` table links accounts to rules by type (source/target)

### Processing Flow
1. **Discovery**: Find rules by account type via `recon_rule_account` table
2. **Filtering**: Apply trigger conditions to staging entry
3. **Selection**: Choose highest priority rule among matches
4. **Execution**: Route to appropriate strategy implementation
5. **Result**: Create/evolve transactions and search expectations

## Testing Gotchas

- **Status Transitions**: Test invalid status changes are rejected
- **Concurrent Processing**: Test race conditions when processing same staging entry
- **Balance Updates**: Verify correct debit/credit components updated based on entry type and status
- **Balance Reversal Testing**: When testing entry archival:
  - Verify `update_account_balance_to_archived` correctly reverses original balance impact
  - Test that only status changes trigger balance reversal (no double-reversal)
  - Ensure Posted entries reverse both posted and pending balances
  - Ensure Pending/Expected/Mismatched entries only reverse pending balances
- **Soft Delete Queries**: Ensure discarded entries don't appear in normal queries
- **Currency Precision**: Test with currencies having different decimal places
- **JSON Serialization**: Test complex types serialize/deserialize correctly with custom Diesel traits
- **Processing Mode**: Test both Transaction and Confirmation mode behaviors
<!-- AI-UPDATED: 2025-09-04 - Staging entry versioning testing gotchas -->
- **Versioning Testing**: Critical considerations for testing versioned staging entries
  - **Primary Key Usage**: All test endpoints must use `staging_entry.id` (primary key) not `staging_entry_id`
  - **Latest Version Lookup**: Tests checking final status must query latest version via list endpoint + filter by `staging_entry_id` + `max_by_key(version)`
  - **API Response Validation**: Verify both `id` and `version` fields present in all staging entry responses
  - **Version Progression**: Test that version numbers increment correctly (1 → 2 → 3) across status changes
  - **Archive Status**: Verify old versions have `status: Archived` and `discarded_at` is set
  - **Helper Function Updates**: Test helper functions must return full `StagingEntryResponse` objects, not just IDs

<!-- AI-UPDATED: 2026-05-13 - assertion now compares (account_name, source_value) pairs and verifies hashed_source_value -->
- **Search Expectation Assertion is a Vec of `ExpectedSearchExpectation` (account + source value)**: `ReconTestEntryExpectedResultBuilder::expected_search_expectations(Vec<ExpectedSearchExpectation>)` asserts both *which* target accounts have an active expectation and *what* their hashed source values are.
  - **`Some(vec![])`** → asserts the endpoint returns `200 []` (no live expectations).
  - **`Some(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])`** → asserts `200` with exactly 2 rows. For each row, `target_account_id` must match the resolved account_id for the fixture name *and* `hashed_source_value` must equal `recon::utils::recon_engine::hash_string(source_value)`. Multiset semantics on the resolved `(account_id, hash)` pair.
  - **`None`** → skip the assertion entirely.
  - **Source value depends on the rule's search_identifier `source_field`**:
    - `OneToOneSingleSingleV1`, `OneToOneSingleManyV1`, `OneToManySingleSingleV1`, `OneToOneSingleSingleV1WithTolerance` use `source_field: "order_id"`. Since `StagingEntryData::recon_staging_entry` hardcodes `order_id: "order_123"`, every callsite for these rules passes `"order_123"`.
    - `OneToOneManyManyV1`, `OneToOneManySingleV1` use `source_field: "metadata.batch_id"`. Pass the source entry's `batch_id` value from the test case (e.g. `"BATCH-001"`, `"BATCH-002"`). All entries in a single case share the same source batch_id, so all callsites in that case use the same string.
    - Check `recon_rule_test::create_recon_rule_request` to see each rule type's configured `source_field`.
  - **Test-fixture names → real account_ids**: the runner builds an `account_name → account_id` map from the accounts created at setup and translates before comparing against the API's real `target_account_id`. A typo in the fixture name panics with `Test fixture error: account_name {?} not found in created accounts`.
  - **Why this shape, not statuses or account-name-only**: every returned row is `status = Expected` (filtered at the DB layer), so status carries no information. The `(account, source-value-hash)` pair fully identifies *which* expectation, and verifying the hash catches encoding bugs (wrong field used as source, hash function changed, etc.) that an account-only check would miss.

## Critical Paths (Moved from main CRITICAL-PATHS.md)

<!-- AI-UPDATED: 2025-06-26 - Moved reconciliation-specific content here -->

### Reconciliation Flow

- **Design**: Staging entries → Processing → Final entries
- **Critical**: Staging entries MUST specify processing_mode (Transaction/Confirmation)
- **Design**: Processing mode determines reconciliation logic path
- **Design**: Entries link to transactions via transaction_id
- **Design**: Transactions can have multiple entries (one-to-many)
- **Design**: BalanceDirection (Credit/Debit) implements double-entry
- **Design**: Search expectations track source/target accounts and key-value pairs for matching

### Recon Rules Flow

- **Design**: Rules define how to match and reconcile entries between accounts
- **Design**: Rules have sources (where to look) and rule targets (what to match against)
- **Design**: Rule execution will trigger reconciliation processes (future)
- **Critical**: See ai-docs/domains/recon-rules/ for detailed implementation notes

## Critical Implementation Gotchas
- **Amount Calculation Bug**: Fixed critical bug in ManySingle strategy where remaining amount calculation was inverted
  - When $10 expected and $5 comes: system was calculating $15 remaining instead of $5
  - Root cause: Logic was adding amounts when entry types are same, should have been subtracting
  - Fix: Inverted if/else logic - same type → subtract, different type → add
  - Impact: Prevents financial discrepancies and ensures proper reconciliation
- **Grouping Field Required**: ManySingle strategy requires valid grouping field for source aggregation
- **Partial Payment Logic**: SingleMany strategy must handle partial payments correctly without negative remaining amounts

### Rule Orchestration Gotchas

<!-- AI-UPDATED: 2025-06-26 - Non-obvious business logic constraints -->

- **Processing Decision**: Based solely on staging entry's processing_mode - no complex rule matching yet
- **Status Immutability**: Processing decision doesn't update staging entry status - caller must handle
- **Profile Validation**: Always verify staging entry belongs to authenticated profile before processing
- **Synchronous Only**: Rule orchestration function is synchronous - no await needed despite AppState param
<!-- AI-UPDATED: 2025-06-27 - Staging entry processed check -->
- **Processed Status Check**: Staging entries with status=Processed are rejected with StagingEntryAlreadyProcessed error

### ProcessAsTransaction Flow Gotchas

<!-- AI-UPDATED: 2025-06-26 - Discovered during ProcessAsTransaction implementation -->

- **Field Extraction Returns Option**: `extract_field_value` returns `Option<String>` not `Result` - can't distinguish between "field doesn't exist" vs "field is null"
- **Metadata Field Access**: Use dot notation for nested fields (e.g., "metadata.order_id", "metadata.payment.method")
- **String Conversion**: All field values converted to String regardless of original type (numbers, booleans, etc.)
- **NoRulesFound Is Valid**: Having no matching rules is a valid business case, not an error - handle gracefully
- **Entry Type Reversal**: Target entries MUST have opposite BalanceDirection (Credit ↔ Debit) from source
- **Direct vs Expected Creation**: Direct entries (source) are infallible, Expected entries (target) can fail if required fields missing
- **Rule Priority**: When multiple rules match, highest priority (u8) wins - no tie-breaking logic
- **Empty Sources Valid**: No sources for an account is valid - means no rules apply to that account
- **Amount Type Mismatch**: API uses FloatMajorUnit but internal processing uses MinorUnit - conversion happens at boundaries
- **ProcessingError Usage**: Represents business logic failures (missing fields, no rules) not system errors

### ProcessAsTransaction Implementation

<!-- AI-UPDATED: 2025-06-30 - Complete implementation of ProcessAsTransaction flow -->

- **Complete Flow**: Staging Entry → Rule Matching → Transaction Creation → Status Update
- **Rule Matching Steps**:
  1. Find all sources for staging entry's account
  2. Filter sources by trigger evaluation
  3. Get rules by IDs and select highest priority
  4. Fetch rule targets for selected rule
- **Transaction Creation**:
  - Creates main entry from staging entry (Direct type)
  - Creates expected entries for each rule target with opposite BalanceDirection
  - Generates search expectations linking source to rule target accounts
  - All created atomically in database transaction
- **Currency Validation**: All accounts (source and rule targets) MUST have same currency - fails with TargetAccountCurrencyMismatch
- **Search Expectations**: Automatically created for each expected entry with:
  - Source and target account IDs
  - Source field/value and target field from SearchIdentifier
  - ExpectationStatus::Expected
- **Status Updates**:
  - Success: Staging entry → Processed, Transaction created
  - Failure: Staging entry → NeedsManualReview, Error reason returned
- **Response Format**: ReconTriggerResponse enum with Success(transaction) or Failure(reason)
- **Atomic Operations**: Uses database transaction - all succeed or all rollback
- **Account Validation**: Batch validates all rule target accounts belong to profile

### ProcessAsConfirmation Implementation

<!-- AI-UPDATED: 2025-02-07 - Complete implementation of ProcessAsConfirmation flow -->

- **Complete Flow**: Staging Entry → Multi-Target Expectation Search → Match Rule Validation → Transaction Evolution → Status Update
- **Multi-Target Expectation Search**:
  - Searches across ALL recon rule targets for the account (not just one)
  - Builds search criteria tuples: (rule_id, target_key, source_value)
  - Uses `find_for_multiple_targets()` for efficient batch search
  - Returns FIRST matching expectation found across all rule targets
- **Match Rule Validation**:
  - Applies versioned match rules (currently V1 only)
  - Compares staging entry fields (target_field) vs expected entry fields (source_field)
  - Currently only supports MatchOperator::Equals
  - Returns MatchRuleValidation::Passed or Failed
- **Transaction Evolution Pattern**:
  - Archives old transaction/entries with status "Archived" (NOT discarded)
  - Creates new transaction with incremented version number
  - Maintains same transaction_id but new version
  - All operations atomic - archive old + create new in single DB transaction
- **Entry Status Priority Logic**:
  - **Highest Priority**: Match failed OR already has mismatched entries → EntryStatus::Mismatched, TransactionStatus::Mismatched
  - **Second Priority**: Match passed but has remaining expected entries → EntryStatus::Pending, TransactionStatus::Expected
  - **Lowest Priority**: Match passed, no expected entries, no mismatches → EntryStatus::Posted, TransactionStatus::Posted
- **Expected ID Mapping**:
  - Maps old expected entry IDs to new entry IDs for continuity
  - Only entries with EntryStatus::Expected get mapped
  - Critical for maintaining expectation chain across transaction versions
<!-- AI-UPDATED: 2026-05-04 - expectation linkage moved from per-entry to per-transaction -->
  - **Linkage**: New search expectations link to the new transaction via `primary_transaction_id` (= new `TransactionNew.id`), NOT via per-entry `expected_id` — the entry-level id mapping is now only used for entry archival, not for expectation linkage
- **Search Expectation Updates**:
  - Creates new search expectations for remaining expected entries
  - Linked to the new transaction version via `primary_transaction_id`
  - Preserves original rule_id, account relationships, and search criteria
  - Sets ExpectationStatus::Expected for new expectations
- **Archive vs Discard Pattern**:
  - Uses TransactionStatus::Archived and EntryStatus::Archived (not soft delete)
  - Preserves original status in discarded_status field
  - Allows transaction evolution history tracking
  - Different from standard soft delete pattern used elsewhere
- **Atomic Evolution Operations**:
  1. Archive old transaction → Archived status
  2. Archive all old entries → Archived status
  3. Archive old search expectations → Archived status
  4. Update staging entry → Processed status
  5. Insert new entries (remaining + current)
  6. Insert new search expectations (for remaining expected entries)
  7. Insert evolved transaction (incremented version)
- **Currency and Validation**: Inherits same currency validation as ProcessAsTransaction
- **Error Handling**: NoExpectationEntryFound when no matching expectations across all rule targets

### Batch Processing and Processor Status

<!-- AI-UPDATED: 2025-01-04 - Added batch processing implementation -->

- **Processor Status Control**: Uses `ReconProcessorStatus` enum (Running/Stopped) with tokio watch channel for real-time control
- **Batch Processing Task**: Async task (`recon_processor`) that continuously processes pending staging entries when status=Running
- **Batch Size Limit**: Processes 20 staging entries per iteration to prevent overwhelming the system
- **Processing Interval**: 5-second sleep between batches when Running, immediate response to status changes when Stopped
<!-- AI-UPDATED: 2025-01-08 - Conditional delay behavior -->
- **Empty Queue Delay**: When no pending staging entries found, processor waits 5 seconds before next iteration (prevents tight loops)
- **Error Delay**: When listing staging entries fails, processor waits 5 seconds before retry (prevents rapid error loops)
- **Concurrent Processing**: Uses `futures::future::join_all` to process batch entries concurrently
- **No Auth Context**: Batch processor runs without authentication - uses `find_account_by_id` instead of profile-scoped queries
<!-- AI-UPDATED: 2025-07-18 - Secured processor status endpoints -->
- **Status API Endpoints**: GET/POST `/recon_engine/processor/status` endpoints now use `AdminApiAuth` - requires admin API key
- **Channel Communication**: Uses `tokio::sync::watch` for efficient status broadcasting - processor subscribes to changes
- **Error Resilience**: Individual staging entry failures don't stop batch processing - errors logged but processing continues
- **State Persistence**: Processor status NOT persisted - defaults to Stopped on restart (safety measure)
- **Core Function Extraction**: `process_staging_entry_core` extracted for reuse between API and batch processor
- **Profile Agnostic Query**: `list_all_staging_entries` bypasses profile isolation - security consideration for batch processing

### Duplicate Entry Detection

<!-- AI-UPDATED: 2025-01-07 - Implemented duplicate transaction detection using search expectations -->

- **Transaction Mode Duplicates**: Uses `search_expectations` table and `idx_expectation_source` index to detect duplicate transactions
- **Detection Logic**: Before creating search expectations, checks if expectation already exists with same (source_account_id, source_key, source_value)
- **Efficient Lookup**: Leverages existing composite index for O(1) duplicate detection - no new indices needed
- **Error Handling**: Returns `ProcessingError::DuplicateEntry` which sets staging entry to `NeedsManualReview` status
- **Confirmation Mode**: No special handling needed - duplicate confirmations naturally fail with `NoExpectationEntryFound`
- **Batch Processing Safe**: Duplicate detection prevents concurrent processing of same transaction in batch processor
- **Search Identifier Based**: Uses same identifiers (e.g., order_id) used for reconciliation matching - ensures consistency

### Search Duplicates Implementation

<!-- AI-UPDATED: 2025-08-11 - search_duplicates with encryption and hashing implementation -->

- **Dual-Storage Design**: search_duplicates table uses encrypted storage with hash-based duplicate detection
- **Security Model**: Raw field values encrypted using `Encryption` type, hashes used for duplicate detection with `Secret<String>` masking
- **Query Methods**: `list_by_staging_entry_id`, `find_by_id`, `list_by_transformation_id_field_path_hashed_field_value` (uses hashed_field_value)
- **Business Rules**: Only one duplicate per (transformation_id, field_path, hashed_field_value) combination via unique constraint
- **Data Protection**: No raw merchant data stored in duplicate table - only encrypted values and masked cryptographic hashes
- **Hash Algorithm**: SHA256 with input format `transformation_id:field_path:raw_value` for scoped uniqueness
- **Masking Implementation**: hashed_field_value uses `masking::Secret<String>` from hyperswitch to prevent accidental exposure in logs
- **Domain Model**: `recon_domain_models::search_duplicate::SearchDuplicate` provides decryption interface with `EncryptableName` for field_value
- **Conversion Pattern**: Follows same encryption/decryption patterns as business_profile and merchant_account domain models
- **Encryption Identifier**: Uses `Identifier::ReconMerchant(merchant_id)` for proper merchant-scoped encryption context
- **Multiple Records**: Can have multiple records per staging_entry_id for different field_path values
<!-- AI-UPDATED: 2025-09-04 - search_duplicates status implementation -->
- **Status Management**: Added SearchDuplicateStatus enum (Active, Archived) for lifecycle management
- **Void Integration**: When voiding staging entries, related search duplicates are archived to prevent orphaned data
- **Batch Operations**: `batch_update_status_by_staging_entry_ids()` for efficient bulk status updates during void operations
- **Database Schema**: Added status column with default 'active', indexed for query performance
- **Archival Pattern**: Follows same soft-delete pattern as other entities - status changes to 'archived' instead of hard deletion

### Search Expectations Encryption Implementation

<!-- AI-UPDATED: 2025-08-18 - search_expectations encryption and hashing implementation -->

- **Encrypted Source Values**: search_expectations now stores `encrypted_source_value` (BYTEA) instead of plain text, following same pattern as search_duplicates
- **Hash-Based Lookups**: Database queries use `hashed_source_value` (VARCHAR(255)) for matching - raw values never used in WHERE clauses
- **Hash Format**: SHA256 with input `rule_id:target_key:source_value` for scoped uniqueness across different recon rules
- **Secret Masking**: `hashed_source_value` uses `masking::Secret<String>` to prevent hash exposure in error logs and debugging output
- **Merchant-Scoped Encryption**: Uses `Identifier::ReconMerchant(merchant_id)` for encryption context, requiring merchant_id column in search_expectations
- **Index Management**: Migration drops old `source_value` indexes before schema changes to prevent conflicts
- **Process Confirmation Impact**: Confirmation flow now generates hashes for lookup instead of using plain text values - `find_for_multiple_targets` updated
- **Manual Transaction API**: Manual transaction creation requires async encryption, parameters reordered to `state: &AppState, auth: &AuthContext` first
- **Database Migration Pattern**: Remove plain text column after adding encrypted fields - ensures no dual storage of sensitive data
- **Domain Model Conversion**: `recon_domain_models::SearchExpectation` provides same decryption interface pattern as other encrypted entities

<!-- AI-UPDATED: 2025-01-22 - Void transaction impact on search expectations -->
### Search Expectations Lifecycle Management

- **Void Transaction Integration**: Transaction voiding automatically archives related search expectations to prevent orphaned reconciliation data
- **Expected Entry Filtering**: Only entries with `EntryStatus::Expected` have associated search expectations that need archiving
- **Batch Archiving Pattern**: Uses `batch_update_status_by_primary_transaction_ids()` for efficient bulk status updates during void operations
- **Status Transition**: Search expectations transition from `ExpectationStatus::Expected` to `ExpectationStatus::Archived` 
- **Consistency Guarantee**: Search expectation archiving wrapped in same database transaction as transaction voiding for atomic consistency
- **Orphan Prevention**: Archiving prevents search expectations from remaining in Expected status when their target entries are voided
- **Memory Management**: Prevents accumulation of unmatched expectations that could impact reconciliation performance

<!-- AI-UPDATED: 2026-05-11 - no-archive on Matched re-routing semantics -->
### No-Archive on Matched: Routing After Match

- **Design intent**: SingleMany / ManySingle / ManyMany keep their `search_expectation` and/or `search_group` Active even after a Matched outcome, so late-arriving entries can evolve the SAME transaction instead of orphaning a new one. SingleSingle is the deliberate exception (1 source + 1 target = nothing more expected, so archive is correct).
- **Implementation**: `discard_transaction_with_entries_and_expectations` still archives the OLD search artifacts on every confirmation. The "no-archive" effect is achieved by `determine_status` (or equivalent) producing a fresh `SearchExpectationCreation` / `SearchGroupCreation` for `Good(*)` outcomes, linked to the NEW transaction version's id. Net state: one Active row at any moment, but the underlying table accumulates one Archived row per evolution.
- **Per-flow rules** (post no-archive change):
  - `OneToOneSingleSingleV1` — `search_expectation: Some` only on `NoTarget`. Archived after Matched. **Intentional, unchanged by this work**: 1 source + 1 target means nothing more to route.
  - `OneToOneSingleManyV1` — `search_expectation: Some` for every outcome except `CurrencyMismatch` (now includes `Good(Balanced)` and `Good(BalancedWithTolerance)`). Late target stagings after a Matched outcome therefore route into the same transaction via the kept-active row.
  - `OneToOneManySingleV1` — `search_group: Some` for every outcome except `CurrencyMismatch` (now includes Matched). `search_expectation` behavior unchanged — only created on `NoTarget`, because the target side is single.
  - `OneToOneManyManyV1` — both `search_group` and `search_expectation: Some` for every outcome except `CurrencyMismatch` (now includes Matched on both).
- **Consequence**: a fully Matched transaction is now reachable from both source-side (via search_group) and target-side (via search_expectation) routing. Late entries reopen it to `OverAmount(Expected)` / `UnderAmount(Expected)` instead of orphaning a new transaction. This is the entire point of the no-archive change.

<!-- AI-UPDATED: 2026-07-20 - ManySingle no-target source append fast path -->
### ManySingle Expected Source Append

- **Configuration**: `source_accumulation_mode` defaults to `versioned`; only `incremental_while_expected` enables the append path.
- **Scope**: Only `OneToOneManySingleV1` transaction-mode entries joining an existing search group whose current transaction data is exactly `V3::Expected` use the append path. All other states and strategies retain full reconstruction.
- **Why this state is special**: `V3::Expected` means no target entry has arrived. Existing sources stay `Pending`, the search group/expectation remain valid, and only the residual Expected amount changes.
- **Versioning exception**: Source-only appends intentionally keep the same transaction primary id and version. The new source entry is inserted at that version, the residual Expected entry and transaction aggregate are updated in place, and staging-entry copy-and-replace versioning still records each consumed input. The next non-Expected transition uses normal transaction versioning.
- **Atomicity**: The commit locks the Expected transaction and residual Expected entry, revalidates eligibility, inserts the source/duplicate/searchable rows, adjusts Expected balances, and updates the transaction in one database transaction. Full reconstruction archives with optimistic `last_modified_at` and `entry_ids` snapshot predicates so a context prepared before an append cannot silently drop that appended source.
- **Fallback**: Missing/multiple residual expectations, wrong target account, non-Expected transaction data, and every target-side staging entry use the existing O(n) reconstruction path.
- **Search artifacts**: The active search group and search expectation are not recreated because their `primary_transaction_id` remains unchanged.
- **Performance boundary**: This removes entry archival/cloning, duplicate churn, decryption of historical entries, and searchable-key re-emission. Updating PostgreSQL's growing `entry_ids` array still rewrites that array; a normalized membership table is required for a strictly O(1) append.

<!-- AI-UPDATED: 2026-09-02 - 10k entry cap on the resolved entry set -->
### Entry Count Cap Per Transaction

- **`Expected` entries are placeholders**: `build_image` drops them and `validate_entries` regenerates one while the transaction is unbalanced. So a confirmation swaps a placeholder for a real entry (count flat), while a source joining via search group adds one *and* keeps the placeholder (count +1).
- **Hence the cap is on the resolved set**, checked in each growable flow's `determine_status` — not on the pre-existing row count, and not at entry load, which `create_transaction_context_without_search_group` and `create_incremental_transaction_context` bypass. Over 10k the staging entry parks in `NeedsManualReviewInternalError`.
- **Why cap at all**: every version re-inserts the whole entry set (`get_new_transaction_data`), so count drives per-confirmation write cost.
- **Not applied in `one_to_many_account/one_to_one`**: that flow rebuilds no image (no `build_image`, no classification step) — it patches the matched `Expected` entry in place via `find_expected_entry`, so there is no resolved `Vec<EntryDataWithClassification>` to count. Its entry count is fixed at 1 source + configured targets regardless.

<!-- AI-UPDATED: 2026-05-13 - all DB-layer search_expectation lookups now filter to Expected -->
### Search Expectation Lookups: All Filter to `status = Expected`

- **Every DB-layer helper filters to `status = Expected`** so Archived rows can never leak into routing, audit, or listing paths. Helpers and the method names that encode the filter:
  - `find_expected_expectations_by_primary_transaction_id_and_target_account_id` (single-row routing lookup)
  - `find_for_multiple_targets` (rule-orchestration batch lookup)
  - `list_by_target_account_id_rule_id_target_key_hashed_source_value` (matching-engine index, `idx_expectation_match`)
  - `list_expected_by_primary_transaction_id` (powers the public listing endpoint)
- **Trait vs diesel naming**: the higher-level `SearchExpectationInterface::list_expected_expectations_by_primary_transaction_id` calls into the diesel `SearchExpectation::list_expected_by_primary_transaction_id`. Keep them aligned — they differ only in their disambiguation (trait disambiguates entity type, diesel name encodes the filter).
- **Why the uniform filter**: Archived expectations represent superseded transaction versions. Surfacing them anywhere — routing, audit listings, or test introspection — risks routing decisions or assertions against stale state. If a future caller genuinely needs Archived rows (e.g. forensic audit), add a separate `_with_archived` method rather than relaxing the filter on an existing one.

<!-- AI-UPDATED: 2026-05-05 - construction-order pattern eliminates FK-mismatch by design -->
- **Primary-Transaction-ID Linkage**: `SearchExpectationNew.primary_transaction_id` MUST equal the `TransactionNew.id` of the transaction that owns it. There is no DB-level FK; divergent ids do not error on insert and only surface later as silent empty results from `find_expected_expectations_by_primary_transaction_id_and_target_account_id` / `batch_update_status_by_primary_transaction_ids`.
  - **The pattern that prevents this**: split entry construction from expectation construction so there is only one source of `primary_transaction_id`. In `recon/utils/transaction.rs`:
    - `Vec<TransactionEntry>::get_new_entries(merchant_id, encryptor) -> (Vec<EntryNew>, DieselTransactionData)` — does not know or allocate `primary_transaction_id`
    - `Vec<TransactionEntry>::get_new_search_expectations(merchant_id, encryptor, primary_transaction_id: String) -> Vec<SearchExpectationNew>` — primary_transaction_id is supplied by the caller, after the owning transaction is built
    - `TransactionNew::new(&entries, ...)` — self-allocates `id`
  - **Required call order**: build entries → build transaction (its `id` is self-allocated) → build search_expectations using `transaction_new.id.clone()`. This is what `core/transaction.rs::create_transaction` does.
  - **Anti-pattern**: do NOT make `get_new_entries` and `TransactionNew::new*` BOTH self-allocate ids without coordinating. Two independent calls to `generate_id_with_default_len("txn")` produce different nanoids, so the search_expectations end up pointing at an id that no transaction has — silent on insert, broken on lookup.
  - **`new_versioned` exception**: `TransactionNew::new_versioned(previous, update, modified_by)` self-allocates `id` internally and is *only* called from flows that don't create new search_expectations (`manual_void_transaction_core`, `manual_post_transaction_core`, `force_reconcile_transaction`). If you add a new versioning flow that needs new search_expectations, you MUST coordinate ids — either by reading them back from `db_transaction.id` after insert and building expectations afterwards, or by switching that flow to `TransactionNew::new` and `get_new_search_expectations`.
<!-- AI-UPDATED: 2026-05-05 - lookup helper lives on DomainTransaction, mirrors the FK -->
  - **Lookup helper placement**: `DomainTransactionExt::fetch_search_expectation_from_db(&self, state, target_account_id, merchant_id) -> ProcessingResult<DomainSearchExpectation>` lives on `DomainTransaction`, not on `DomainEntry`, in `recon/src/domain/recon_engine/utils.rs`. The lookup is keyed by `(self.id, target_account_id)` — i.e. `(primary_transaction_id, target_account_id)` — so it conceptually belongs to the transaction even though a contributor coming from "this entry is the expected one" might reach for `DomainEntry` first. If you find yourself wanting to fetch a search expectation, get a `&DomainTransaction` and call this method; entries don't own the FK.

<!-- AI-UPDATED: 2025-08-12 - Duplicate detection flow and transformation config scalability -->
- **Processing Integration**: Duplicate detection occurs BEFORE rule orchestration via `check_for_duplicate()` function
- **Constraint-Based Detection**: Uses direct database insertion with "A unique constraint violation occurred" string matching
- **Error Flow Integration**: Duplicate detection returns `ProcessingError::DuplicateEntry` which routes through existing error handling
- **Staging Entry Updates**: Duplicates automatically set staging entry status to `NeedsManualReview` via `process_error_context`
- **Scalable Field Access**: Transformation configs now provide `unique_field()` method eliminating need for enum matching
- **Simplified Architecture**: No separate context objects - direct validation function that either succeeds or returns appropriate error

### Recon Rule Context Storage

<!-- AI-UPDATED: 2025-07-15 - Implemented rule_id context storage in transactions table -->

- **Rule Context Tracking**: All transactions now store `rule_id` to track which recon rule processed the data
- **Database Schema**: Added non-nullable `rule_id VARCHAR(64)` column to transactions table with index for query performance
- **Transaction Mode**: Stores the selected highest-priority rule's ID that processed the staging entry
- **Confirmation Mode**: Preserves the original rule_id from the evolved transaction to maintain rule context continuity
- **Manual Transactions**: Use placeholder `"manual_transaction"` rule_id for transactions created via API (not recon processing)
- **API Response**: All transaction responses include `rule_id` field for transparency and debugging
- **Audit Trail**: Enables queries like "show all transactions processed by rule X" for compliance and analysis
- **No Foreign Key**: Uses simple string reference without foreign key constraint for flexibility
- **Index Performance**: `idx_transactions_rule_id` index enables efficient rule-based transaction queries
- **Backward Compatibility**: Migration adds rule_id as required field - existing data handled via database resurrection

### Staging Entry with Account Join Pattern

<!-- AI-UPDATED: 2025-08-29 - StagingEntryAccountView join pattern implementation -->

- **StagingEntryAccountView**: Efficient joined query for staging entries with account data
  - Single database call instead of N+1 queries for staging entry + account lookups
  - Used in `list_staging_entries_by_profile_id()` to include account information in response
  - Provides account name and account ID alongside staging entry data
- **Database Query Pattern**: Inner join between staging_entries and account tables
  - Join condition: `staging_entries.account_id = account.account_id`
  - Filtered by profile_id for tenant isolation in list operations
  - Supports all existing filters (status, entry_type, currency, account_id, transformation_history_id, time_range)
- **API Enhancement**: Staging entry list endpoint now returns account context
  - Response includes `account: {account_id, account_name}` field for better UX
  - Maintains backward compatibility by adding account field to existing response
  - Single/individual staging entry endpoints also include account data via separate lookup
- **Performance**: Optimized queries use single join instead of N+1 pattern for list operations
  - Critical for staging entry lists with many entries across different accounts
  - Individual operations (create/get-by-id) still use separate account lookup for simplicity
- **View Struct Pattern**: `StagingEntryAccountView` struct combines `StagingEntry` and `Account` models
  - Follows same pattern as `IngestionConfigHistoryView` and `TransformationConfigHistoryView`
  - Maintains separation of concerns while enabling efficient joined queries

### Resolution Mapping System

<!-- AI-UPDATED: 2025-01-09 - Implemented Resolution Provider trait and action mapping in exception_management domain -->

- **ResolutionProvider Trait**: Centralized trait in `crates/recon/src/domain/exception_management/resolution_mapping.rs` for mapping exception types to available resolution actions
- **Domain Organization**: Part of `exception_management` domain module with `resolution_mapping` as submodule - follows proper domain hierarchy
- **Data Structure Integration**: Implements trait on all data types with exceptions - `StagingEntryData`, `TransactionData`, and their nested types
- **Action Types**: Uses `ResolutionActionType` enum from `common_enums` with 4 actions: `CreateRule`, `CreateAccount`, `VoidStagingEntry`, `VoidTransaction`
- **Direct Delegation**: `StagingEntryDataV1` directly accesses `error_type` from `ManualReviewData` for efficiency (no intermediate trait layer)
- **Versioned Design**: All implementations handle versioned enums (V1 pattern) for future compatibility
- **Centralized Logic**: All resolution mapping logic in single domain module prevents scattered business rules across codebase
- **No Dependencies**: Uses foundational `common_enums` crate to avoid circular dependencies between `common_types` and `api_models`
- **Clippy Compliance**: Module structure follows clippy requirements (no mod.rs files) with exception_management.rs declaring submodules

### Search Duplicates Revamp Implementation

<!-- AI-UPDATED: 2025-01-22 - Major architectural change from constraint-based to detection-based duplicate handling -->

- **Constraint to Detection Migration**: Moved from unique database constraint to explicit duplicate detection with user choice
  - Removed `uniq_idx_search_duplicates` unique constraint to allow multiple duplicate records
  - Added `idx_search_duplicates_lookup` for efficient duplicate detection queries
  - Changed from "prevent duplicates" to "detect and offer choice" model
- **Deferred Duplicate Creation**: Search duplicate entries now created ONLY after successful staging entry processing
  - `insert_search_duplicate()` called in same transaction as status update to Processed
  - Prevents orphaned duplicate records when processing fails
  - Atomic operation ensures data consistency
- **Force Processing Implementation**: Added bypass mechanism for duplicate detection via `/{id}/process_duplicate` endpoint
  - Only works for staging entries in `NeedsManualReview` status with `DuplicateEntry` error type
  - Other manual review types (missing fields, etc.) return `InvalidForDuplicateProcessing` error
  - Bypasses duplicate check via `skip_duplicate_check` parameter in core processing logic
- **Detailed Error Messages**: Duplicate error messages provide comprehensive context to users
  - Format: "Duplicate entry detected for field '{field_path}' with value '{field_value}'. Found {duplicate_count} existing duplicate(s): {duplicate_staging_entry_ids:?}"
  - Includes field path (e.g., "metadata.merchant_reference"), actual field value, duplicate count, and staging entry IDs
  - `ProcessingError::DuplicateEntry` contains all context fields for detailed user feedback
  - User-facing error messages include actionable information for troubleshooting and decision-making
- **Resolution System Integration**: Added `ForceProcessStagingEntry` as new resolution action type
  - Integrated into resolution mapping system for duplicate entries
  - Provides structured way for exception management to offer force processing option
- **Duplicate Detection Timing**: Check occurs BEFORE rule orchestration and normal processing
  - Fails fast if duplicates exist, preventing unnecessary processing overhead
  - Same error handling flow as other processing errors (routes to manual review)
- **Test Pattern Updates**: All tests now properly create transformation configs instead of using hardcoded IDs
  - Test helper functions accept `profile_id` parameter for clean transformation config creation
  - Progressive duplicate tests verify detection works across multiple staging entries with same unique value
<!-- AI-UPDATED: 2025-01-22 - Resolution mapping endpoints require domain model decryption -->
- **Resolution Endpoints Require Decryption**: `/exception_management/staging_entries/{id}/resolutions` and `/exception_management/transactions/{id}/resolutions` endpoints must decrypt data to access business logic via `ResolutionProvider` trait
<!-- AI-UPDATED: 2025-10-06 - Link staging entries to transaction resolution implementation -->
- **Link Staging Entries to Transaction Resolution**: New bidirectional resolution method for manual reconciliation
  - **Purpose**: Replace expected entries in transactions with staging entries when automated processing fails
  - **Use Case**: OMS creates transaction with expected PSP entries, PSP data fails processing, manual intervention needed
  - **Bidirectional Approach**: Available from both transaction (select staging entries for expected entry) and staging entry (select transaction for staging entries) perspectives
  - **Balance Validation**: Combined transaction entries must maintain double-entry balance (credits = debits)
  - **Account Validation**: All staging entries must belong to same account as the expected entry being replaced
  - **Currency Validation**: Staging entries must match expected entry currency for consistency
  - **Status Requirements**: Only works with staging entries in Pending or NeedsManualReview status, expected entries in Expected status
  - **Search Duplicate Integration**: Creates search duplicates for processed staging entries with transformation tracking
  - **Atomic Operations**: Transaction versioning, entry creation, staging entry processing all within single database transaction
  - **Resolution Mapping**: Available as `LinkStagingEntriesToTransaction` action for Pending and NeedsManualReview staging entries and Expected transactions
<!-- AI-UPDATED: 2025-01-08 - Staging entry update/correction functionality -->
- **Staging Entry Update API**: Complete correction functionality implemented via `PUT /staging_entries/{id}` endpoint
  - New `StagingEntryUpdateRequest` struct with all updatable fields (reason, account_id, entry_type, amount, currency, effective_at, metadata, transformation_id)
  - Reason field mandatory for audit trail - explains why correction was made
  - Full field replacement pattern - all fields must be provided in update request
- **Correction Restrictions**: Update/correction functionality restricted to `NeedsManualReview` status only
  - Validation implemented via `StagingEntryValidationExt` trait on `StagingEntry` model
  - All other statuses (`Pending`, `Processed`, `Void`, `Archived`) return `InvalidStatus` error
  - Exhaustive pattern matching enforces this constraint without wildcards
  - All error types within NeedsManualReview are correctable (NoRulesFound, DuplicateEntry, MissingUniqueField, etc.)
- **Copy-and-Replace Versioning**: Updates create new versions using established versioning pattern
  - Original entry marked as `Archived` status with `discarded_at` timestamp and preserved in `discarded_data`
  - New version created with incremented `version` number and `Pending(Edited)` data containing correction reason
  - API shows updated entries with `Pending` status with `Edited` sub-variant containing the reason
  - Complete audit trail maintained through versioning system
- **Account Validation on Updates**: Account existence and profile ownership validated during updates
  - Account must exist and belong to authenticated user's profile
  - Returns 404 if account not found or belongs to different profile
  - Transformation config validation also performed to ensure valid transformation_id
- **No Search Duplicates for Updates**: Staging entry updates/corrections do not create search duplicates
  - Updates result in `Pending(Edited)` state which is a final state (not processed further by recon engine)
  - Search duplicates only created when entries successfully process to `Processed` status
  - Manual corrections require re-processing if intended to flow through reconciliation again
- **New Resolution Action**: `EditStagingEntry` added to `ResolutionActionType` enum for exception management integration
  - Provides structured resolution option for staging entries requiring manual correction
  - Integrated into resolution mapping system for comprehensive exception handling workflows
- **Comprehensive Test Coverage**: Update functionality includes extensive test coverage
  - Validation tests verify status-based correction restrictions using rstest parametrization
  - Field update tests ensure individual and multiple field corrections work correctly
  - Error handling tests cover profile isolation, non-existent entries, and invalid data scenarios
  - Workflow tests verify complete update lifecycle including versioning and archival behavior
  - Helper function `create_test_staging_entry` supports creating entries with any target status for testing
<!-- AI-UPDATED: 2025-10-11 - Staging entry data V2 restructuring -->
- **Staging Entry Data V2 Restructuring**: Major architectural restructuring to follow transaction pattern with versioned pending data
  - **V1 to Archived Migration**: Moved `StagingEntryDataV1` to `common_types::staging_entry::archived` module for backward compatibility
  - **V2 Structure**: New `StagingEntryDataV2` with hierarchical pending structure: `Pending(PendingData::V1(PendingDataV1))` 
  - **Pending Data Variants**: `PendingDataV1` has two variants:
    - `New`: For initial/new pending staging entries 
    - `Edited(EditedDataV1 { reason })`: For edited pending entries with correction reason
  - **Removed Top-Level Edited**: `Edited` is no longer a top-level status, now a sub-variant of `Pending`
  - **ProcessingErrorType Location**: `ProcessingErrorType` and `ProcessingErrorTypeV1` moved back to main staging_entry module (not archived)
  - **Test Pattern Update**: Test files updated to use `PendingDataV1::New` instead of `Initial`, with separate handling for `Edited` variant
  - **Conversion Logic Restructuring**: Updated trait implementations in `utils/staging_entry.rs` to handle new V2 structure
  - **API Models Flat Structure**: API models remain flat following transaction pattern:
    - `StagingEntryData::Pending(PendingData)` where `PendingData` has `New` and `Edited { reason }` variants
    - Conversion maps V2 structure to flat API structure for consumers
    - No versioning exposed in API - keeps interface simple while internal types are versioned
  - **Modular Conversion Implementation**: Extracted `PendingData` conversion logic into separate trait implementation for better code organization
<!-- AI-UPDATED: 2026-08-10 - Added cursor-paginated linkable staging entry listing -->
- **Linkable Staging Entries APIs**:
  - Legacy `GET /exception_management/transactions/{id}/linkable_staging_entries` remains an unpaginated `Vec<GetStagingEntryResponse>` for compatibility, but is deprecated.
  - `POST /exception_management/transactions/{id}/linkable_staging_entries/v2/list` returns the shared cursor page, supports ID/effective-at sorting and the non-status/non-currency V2 filters, and uses the staging-entry limits (default 10, max 50).
  - The transaction's currency and the six linkable granular manual-review statuses are server-owned filters. Linkability is applied before the keyset window; filtering after pagination creates sparse pages and incorrect cursors.
  - `StagingEntryManualReviewData::is_linkable()` is the source of truth: NoRulesFound, DuplicateEntry, NoExpectationEntryFound, MultipleExpectedEntriesFound, MissingSearchIdentifierValue, and MissingMatchField.

<!-- AI-UPDATED: 2025-11-08 - Staging entry void validation patterns -->
## Staging Entry Void Validation Implementation

- **Voidability Rules**: `is_voidable()` on `DomainStagingEntryStatus` enforces proper void state transitions
  - **Implementation Location**: impl block of `DomainStagingEntryStatus` (`common_types/src/staging_entry/status.rs`) — the hierarchical domain enum, so raw diesel rows convert up before being asked
  - **Voidable Statuses**: Only `Pending` and `NeedsManualReview` statuses can be voided  
  - **Non-Voidable Statuses**: `Processed`, `Void`, and `Archived` statuses cannot be voided (finalized states)
- **Validation Integration**: Void validation occurs in `void_staging_entry()` core function BEFORE database operations
  - **Error Type**: Returns `StagingEntryNotVoidable` error with staging entry ID for clear user feedback
  - **API Error Code**: Maps to 400 Bad Request with error code "RE-410" for client-side handling
  - **Security**: Validation happens after existence and profile ownership checks but before expensive versioning operations
<!-- AI-UPDATED: 2025-10-16 - Search duplicates status column for lifecycle management -->
- **Search Duplicates Status Management**: Added `status` column for proper duplicate lifecycle tracking
  - **Status Types**: `Active` (default) for live duplicates, `Archived` for discarded/replaced duplicates
  - **Query Filtering**: All search duplicate queries filter by `status = 'active'` to exclude archived records
  - **Lifecycle Pattern**: Status changes to `Archived` when entries are discarded, back to `Active` when the same entries are cloned
  - **Database Optimization**: Uses partial indexes `WHERE status = 'active'` instead of broad indexes for better performance
  - **Atomic Updates**: Status changes happen within same transaction as entry operations for consistency
  - **Unique Predicate**: `primary_staging_entry_id` is used as unique predicate to ensure only one active duplicate per primary staging entry. Note that one staging entry (not the primary staging entry) can have multiple active duplicates.

<!-- AI-UPDATED: 2026-06-07 - Exposed/searchable metadata on entries -->
<!-- AI-UPDATED: 2026-06-11 - Kept stateless build_search_entry_keys (engine recomputes from config each version); config/schema lookups batched via list_*_by_ids_and_profile_id -->
## Exposed & Searchable Metadata (Reporting)

- See `transformation/AGENT-NOTES.md` for the schema `exposed` config and the staging side. This covers the entry/recon-engine side.
- **Entries**: `entry` gained ONE unencrypted JSONB column `exposed_metadata` (no searchable column). It is threaded plain (never encrypted) through `RawEntryData` → `DirectTransactionCreationEntry` → `DomainEntry` → `EntryNew`. `from_staging_entry` copies it from the staging entry; `from_domain_entry` copies it from the existing entry so carried-over entries keep it across versions. System-generated expected/tolerance entries use `'{}'`.
- **Searchable rows**: live in a normalized table `search_entry_key` (id, profile_id, entry_id, primary_transaction_id, metadata_key, metadata_value, timestamps) — one row per searchable key-value, indexed `(profile_id, metadata_key, metadata_value)` and `(entry_id)`. No `account_id` column. Append-only; reporting joins to live entries/transactions for status.
- **How rows are built (stateless recompute)**: `build_search_entry_keys(state, &entries, primary_transaction_id, profile_id)` in `recon_engine/utils.rs`. For each entry it resolves searchable keys via that entry's OWN `transformation_id → transformation_config → metadata_schema → exposed_keys().searchable`, then projects the entry's `exposed_metadata` (searchable ⊆ exposed). Both `exposed_metadata` and `exposed_keys()` are keyed by **field_name** (not identifier), so the projection lines up — see the transformation notes for why `exposed_keys()` must return field names. **Why per-entry config**: in one transaction the source and target entries come from different schemas; using a single schema would mislabel one side. **Why recompute every version instead of carrying rows forward**: the engine stays stateless w.r.t. previously-written `search_entry_key` rows — it never reads them back; each transaction version re-derives from the current transformation config.
- **Batched lookups**: the configs and schemas are fetched in TWO queries total, not per-entry — `list_transformation_configs_by_ids_and_profile_id` then `list_metadata_schemas_by_ids_and_profile_id` (`eq_any` on ids, profile-scoped). Any id missing from the result fails the whole call with `ProcessingError::DatabaseError` (naming the missing ids), matching the old per-id `find_*` behavior — a dangling `transformation_id` must fail processing loudly, not silently skip rows.
- **Plumbing**: every flow prep calls `build_search_entry_keys` right after `get_new_transaction_data()` (before `transaction_data.entries`/`.transaction` are consumed by `construct_new`), stores `new_search_entry_keys` on its context struct, and the commit closure calls `SearchEntryKeyNew::batch_insert(context.new_search_entry_keys, &conn)` alongside `EntryNew::batch_insert` — mirrors the `search_expectation` plumbing across all 5 flows (one_to_one/one_to_many/many_to_one/many_to_many + one_to_many_account). Rows are re-emitted per transaction version (new entry_id + primary_transaction_id).
