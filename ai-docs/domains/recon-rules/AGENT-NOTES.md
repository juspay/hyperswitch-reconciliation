# Recon Rules - Agent Notes

## Critical Rules

- **Strategy Schema**: `ReconRule.strategy` JSONB column is the single source of truth for reconciliation logic
- **Versioned Types**: All complex types (Trigger, MatchRules, SearchIdentifier) use versioned enums for backward compatibility
- **Strategy Types**: Currently supports OneToOne variants (SingleSingle, SingleMany, ManySingle)
- **JSON Storage**: Complex types stored as native Diesel types using custom ToSql/FromSql implementations
- **Atomic Creation**: Rules and recon_rule_account rows MUST be created in single transaction
- **Account Validation**: Always validate all referenced accounts exist and belong to authenticated profile
- **Currency Consistency**: All accounts in a rule must have the same currency
- **Account→Rule Mapping**: `recon_rule_account` table provides efficient rule discovery by account type
- **Priority Selection**: Highest priority rule wins when multiple rules match staging entry
- **Trigger V2 Support**: Composite source-account triggers now support flat `all` / `any` logic across multiple conditions

## Common Gotchas
- **JSON Serialization Failures**: Always use `.change_context()` when converting to/from serde_json::Value
- **N+1 Query Problem**: Use batch account validation with HashMap lookups, not individual queries
- **Version Tag Format**: Use explicit `#[serde(rename = "v1")]` tags to prevent breaking changes
- **ID Prefix Consistency**: Sources use "src_" prefix, rule targets use "tgt_" prefix, rules use "rule_" prefix
- **OneToOne Validation**: Source and target accounts cannot be the same in OneToOne relationships
- **Profile Scoping**: All account references must belong to the authenticated profile
- **Data Integrity in List Operations**: Never use `unwrap_or_default()` when grouping sources/targets by rule_id
  <!-- AI-UPDATED: 2025-06-24 - Refactored list function to avoid hiding data integrity issues -->
  - Every rule must have sources and targets - missing data indicates corruption
  - Use explicit error handling: `sources_by_rule.get(&rule_id).ok_or(ReconRuleError::InternalServerError)?`
- **String Parameter Performance**: Use `&str` instead of `String` for function parameters when ownership not needed
  <!-- AI-UPDATED: 2025-06-24 - Performance optimization across all interfaces -->
  - Database interfaces, core functions, and diesel models all use `&str` for ID lookups
  - Reduces memory allocations and improves API flexibility
<!-- AI-UPDATED: 2025-06-24 - Discovered during endpoint implementation -->
- **StorageErrorExt Required**: Must implement `StorageErrorExt<T, ReconRuleError>` trait to use `to_not_found_response()` pattern
- **Batch Query Methods**: Need `list_by_rule_ids` methods in both source and rule target models for efficient multi-rule fetching
- **Profile Isolation in Retrieval**: Even if rule exists, return 404 if it belongs to different profile (security requirement)
<!-- AI-UPDATED: 2025-06-24 - JSON handling changes -->
- **Direct Type Storage**: Trigger, MatchRules, SearchIdentifier stored as native types, not serde_json::Value
- **Macro Implementation**: Uses common_utils::impl_to_sql_from_sql_json! macro for JSON serialization
- **Composite Trigger Validation**: `Trigger::V2` with empty `conditions` must be rejected during rule creation with `InvalidReconRule`
- **Missing Trigger Fields**: Both V1 and V2 condition evaluation return `false` for missing fields; they do not raise processing errors
- **No Nested Trigger Logic Yet**: V2 only supports one boolean layer (`all` / `any` over flat conditions). Do not model nested groups without introducing a new version
<!-- AI-UPDATED: 2026-05-19 - Composite search keys and grouping fields -->
- **Composite Search Keys** (`SearchIdentifierV2::Composite`): Multi-field source→target mapping joined by a `CompositeDelimiter` (`|`).
  - Validation rejects: empty `keys`, duplicate `source_field`, duplicate `target_field`. Single-key composites are accepted but prefer `SearchIdentifierV2::Single` — the composite path produces byte-equivalent output today, but semantically `Single` is correct.
  - **No escaping in composite values**: `build_composite` just joins parts with the delimiter. The delimiter MUST NOT appear in any source/target value, otherwise two distinct input combinations can collide and match the wrong expectation. Caller is responsible for choosing a safe delimiter.
- **Composite Grouping Field** (`GroupingFieldV1::Composite`): Multi-field grouping key. Validation rejects empty `fields` and duplicate fields.
- **Sort Canonicalization Differs By Type — Intentional**:
  - `CompositeSearchIdentifier`: keys sorted by `source_field.to_string()` (display-string order).
  - `CompositeGroupingField`: fields sorted by derived `Ord` on `EntryField` (variant declaration order).
  - These are independent canonicalization because the two values are never compared to each other (search drives expectation lookup; grouping drives source-account batching). Do not "unify" them without auditing both consumers.
- **Source-Field Set Invariant**: For `ManySingle` / `ManyMany`, `validate_source_field_sets` requires:
  - `search_identifier` source-field set must be a **subset** of the `grouping_field` set (equal sets are the common case; a strict subset is allowed).
  - Every `match_rule.source_field` must be in `grouping_field`.
<!-- AI-UPDATED: 2026-07-30 - Match rule label for mismatch reasons -->
- **`MatchRuleV1.label: Option<String>`**: human label for what a match rule compares, surfaced as `FieldMismatchDetail.label` in mismatch data instead of the raw field name. Required struct field, optional value — every `MatchRuleV1 { .. }` literal must set it (even to `None`).
- **`target_key` Column is TEXT, Not VARCHAR**: Migration `2026-05-07-120000` widens `search_expectation.target_key` from `VARCHAR(128)` to `TEXT` to fit composite keys. Down migration may fail if any composite key value exceeds 128 chars — treat the change as one-way once composite rules exist.

## Design Decisions
- **Why Versioned Enums**: 
  - Enables backward compatibility when adding new trigger/match types
  - Allows gradual migration of existing data
  - Type-safe handling of different versions
- **Why JSON Storage**:
  - Flexible schema for complex nested types
  - Avoids creating many normalized tables
  - Enables versioned enum storage
- **Why Relationship Type Enum**:
  - Type-safe validation of different rule patterns
  - Extensible for future relationship types
  - Clear business logic separation
- **Why Constructor Pattern Over Extension Traits**:
  <!-- AI-UPDATED: 2025-06-24 - Refactored from extension traits to constructors -->
  - Simple field mapping doesn't need abstraction layer
  - Logic co-located with data structures in diesel_models
  - More idiomatic Rust for straightforward object creation
  - ID generation handled at call site for better control
- **Why Native Type Storage Over serde_json::Value**:
  <!-- AI-UPDATED: 2025-06-24 - Changed JSON storage approach -->
  - Type safety at database layer
  - No manual JSON conversion in business logic
  - Automatic serialization/deserialization via Diesel traits
<!-- AI-UPDATED: 2025-12-15 - Bidirectional conversion pattern for V2 API -->
- **Why Bidirectional Conversion Traits** (`ReconStrategyExt` and `ReconStrategyApiExt`):
  - Clean separation between domain models (common_types) and API models (api_models)
  - `ReconStrategyExt::to_api_response()` converts domain → API for responses
  - `ReconStrategyApiExt::to_diesel_model()` converts API → domain for requests
  - Associated `ReturnType` ensures compile-time type safety at each layer
  - Trait-based approach enables recursive conversion through nested strategy enums
  - `get_rule_accounts_new()` method on strategy automatically extracts all accounts for `recon_rule_account` table

## Relationships
- **Recon Rule**: Main entity with rule metadata (name, description, priority)
- **Recon Rule Sources**: One-to-many - rules can have multiple source accounts
- **Recon Rule Targets**: One-to-many - rules can have multiple rule target accounts
- **Accounts**: Many-to-many through sources and targets - accounts can be in multiple rules
- **Business Profile**: All rules scoped to a single profile
<!-- AI-UPDATED: 2025-12-04 - ReconRuleAccount relationships -->
- **Recon Rule Accounts**: Separate mapping table `recon_rule_account` linking (account_id, rule_id, account_type)
  - `account_type` is `ReconRuleAccountType` enum (`source` / `target`) stored as DB enum
  - Used for fast lookups of all rules for a given (account_id, role) without scanning all sources/targets
  - Rules can be reachable both via sources/targets and via recon_rule_account; keep mappings consistent when evolving flows

## When Working Here
- **Creating Rules**:
  - Validate rule name and description are non-empty
  - Ensure priority is positive integer
  - Collect all account IDs for batch validation
  - Determine relationship type from source/target counts
  - Apply relationship-specific validation
- **Validating Accounts**:
  - Use `find_accounts_by_profile_id_and_account_ids` for batch lookup
  - Convert to HashMap for O(1) existence checks
  - Ensure all accounts belong to authenticated profile
- **JSON Handling**:
  - Always wrap serde_json operations with error context
  - Test serialization round-trips for all complex types
  - Use JSONB in database for better query performance

## API Behavior
- **Create Recon Rule**:
  - Accepts versioned complex types in request
  - Validates all business rules before database operations
  - Returns 201 Created with full rule details including generated IDs
  - Rolls back entire transaction if any validation fails
<!-- AI-UPDATED: 2024-06-24 - Added new endpoint behaviors -->
- **Get Recon Rule by ID**:
  - Returns 404 if rule doesn't exist OR belongs to different profile (security)
  - Uses `to_not_found_response()` pattern for consistent error handling
  - Fetches sources and targets separately after rule validation
- **List Recon Rules by Profile**:
  - Automatically scoped to authenticated user's profile
  - Uses batch queries to avoid N+1 problems when fetching sources/targets
  - Groups sources/targets by rule_id using HashMap for O(1) lookup
  - Returns empty array when no rules exist (not 404)
<!-- AI-UPDATED: 2024-12-08 - Added update endpoint -->
<!-- AI-UPDATED: 2026-05-14 - priority/is_active updatable; strategy replaced wholesale -->
- **Update Recon Rule** (`PUT /recon_rules/{rule_id}`):
  - Updatable: `rule_name`, `rule_description`, `link_with_rule`, `aging_config`, `priority`, `is_active`, `strategy`. Omitted fields are left untouched.
  - `strategy` is replaced **wholesale** — whatever `ReconStrategy` the caller sends is written as-is, with no validation (account existence, currency consistency, trigger checks are all skipped). This is intentional: the endpoint is currently driven only by trusted internal callers. Validation parity with the create path is deferred.
  - `recon_rule_account` rows are NOT re-synced on a strategy update — a strategy update that changes source/target account_ids leaves that index stale. Callers must not do that until the validation/sync work lands.
  - **Not transactional** — each provided field fires its own DB write and bumps `last_modified_at`, so a multi-field PUT is N writes and a partial failure leaves a partial update.
- **Response Structure**:
  - Includes all sources and targets with their generated IDs
  - Complex types deserialized from JSON for API response
  - Never exposes raw database JSON values

## API Versioning
<!-- AI-UPDATED: 2025-12-04 - Added V2 endpoint with strategy-based response -->
- **V2 Get Recon Rule by ID** (`GET /recon_rules/v2/{rule_id}`):
  - Returns consolidated `strategy` field instead of flat `sources`/`targets` arrays
  - Strategy field matches database `strategy` column structure (single source of truth)
  - Response type: `GetReconRuleResponse` with nested versioned strategy structure
<!-- AI-UPDATED: 2025-12-15 - Added V2 create endpoint -->
- **V2 Create Recon Rule** (`POST /recon_rules/v2`):
  - Accepts `ReconRuleCreateRequest` with consolidated `strategy` field instead of separate `sources`/`targets` arrays
  - Strategy directly maps to versioned `ReconStrategy` enum (e.g., `OneToOne(SingleSingle {...})`)
  - Returns `GetReconRuleResponse` with strategy-based response structure
  - Currency validation enforced: all accounts in strategy must have same currency

## Relationship Types
- **OneToOne**: Single source, single target - prevents same account usage
- **OneToMany**: Single source, multiple targets - future validation TBD
- **ManyToOne**: Multiple sources, single target - future validation TBD  
- **ManyToMany**: Multiple sources, multiple targets - future validation TBD

## Testing Gotchas
- **JSON Round-trips**: Test all complex types serialize and deserialize correctly
- **Relationship Validation**: Test each relationship type rejects invalid combinations
- **Batch Validation**: Verify single query used for multiple account validation
- **Transaction Rollback**: Test partial failures roll back entire rule creation
- **Version Compatibility**: Ensure old JSON data works after adding new enum versions
- **Account Ownership**: Test cross-profile account access is blocked
- **Error Context**: Verify meaningful error messages for JSON serialization failures
<!-- AI-UPDATED: 2025-06-27 - Rule processing test coverage -->
- **Field Extraction Coverage**: Tests must cover standard fields, metadata fields, and nested metadata
- **Missing Field Behavior**: Trigger evaluation returns false for missing fields, not error - test this explicitly

## Performance Considerations
- **Batch Queries**: Always validate accounts in batch, never individually
- **JSON Indexing**: Use JSONB with GIN indexes for version-based queries if needed
- **Transaction Size**: Keep rule creation transactions small and fast
- **Memory Usage**: Build response objects before database transaction to avoid cloning

## Rule Execution and Orchestration

### Discovery Process
1. **Account Lookup**: Find rules by account type via `recon_rule_account` table
2. **Rule Retrieval**: Get complete rule objects by IDs
3. **Trigger Filtering**: Apply trigger conditions to staging entry
4. **Priority Selection**: Choose highest priority rule among matches

### Execution Flow
- **Transaction Mode**: Creates new transactions with expected entries
- **Confirmation Mode**: Fulfills existing expectations and evolves transactions
- **Strategy Dispatch**: Routes to appropriate strategy implementation
- **Error Handling**: Sets entries to `NeedsManualReview` on failures

### Strategy Components
- **Search Identifier**: Maps source fields to target fields for expectation matching
- **Match Rules**: Defines field comparisons for validation (amount, currency, metadata)
- **Trigger**: Field-based conditions that activate the rule
  - `Trigger::V1`: single condition
  - `Trigger::V2`: flat composite condition list with `logic: all | any`
- **Source Account**: Account that triggers the rule (with trigger conditions)
- **Target Account**: Account that receives expected entries

### Field Types
- **Standard Fields**: amount, currency, entry_type, effective_at, order_id
- **Metadata Fields**: metadata.{key} for any nested metadata value
- **Comparison Operators**: equals, not_equals (for triggers)
- **Match Operators**: equals (for match rules)

## API Versioning

### V2 Endpoints
- **Create**: `POST /recon_rules/v2` with strategy-based request
- **Get**: `GET /recon_rules/v2/{id}` with strategy-based response
- **Strategy Field**: Consolidated strategy structure instead of flat sources/targets

### V1 Compatibility
- **Legacy Support**: V1 endpoints continue to work with existing data
- **Migration Path**: Gradual migration to V2 strategy-based structure
- **Backward Compatibility**: Old JSON data remains functional
