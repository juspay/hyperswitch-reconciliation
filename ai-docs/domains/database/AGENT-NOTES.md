# Database - Agent Notes

## Critical Rules
- **Async Diesel Pattern**: All database operations use async diesel with connection pooling
- **StorageResult Type**: All DB operations return `StorageResult<T>` for consistent error handling
- **Timestamps**: Use `common_utils::date_time::now()` for consistent timestamp generation
- **ID Generation**: Use `common_utils::id_generation::generate_id_with_default_len(prefix)`
  - Format: Base62 (A-Z, a-z, 0-9), 20 characters, no special characters
  - Example: `generate_id_with_default_len("account")` → `account_a3D9k2MP8xQ4r7V9`
  - Legacy IDs: Older records may contain `-` or `_` from old nanoid (both formats coexist)
- **Enum Storage**: Custom enums use `db_enum` storage type via `recon_derive::diesel_enum`

## Common Gotchas
- **Connection Pool**: Don't hold connections across await points - use `get_connection().await?`
- **Migration Order**: Enums must be created before tables that use them
- **BIGINT for Amounts**: All monetary values stored as BIGINT to prevent precision loss
- **No CASCADE Deletes**: Relationships don't cascade - handle cleanup explicitly
- **Array Types**: Use `DieselArray<String>` wrapper for Vec<String> fields
- **JSON Storage**: Complex types stored as serde_json::Value require explicit serialization/deserialization
- **JSONB vs JSON**: Use JSONB for queryable JSON fields, JSON for simple storage
- **Versioned JSON Types**: Use tagged unions with explicit version fields for backward compatibility
<!-- AI-UPDATED: 2025-01-21 - JSONB data field pattern for structured error context -->
- **JSONB Data Field Pattern**: Use separate JSONB data column for structured context (error details, void reasons)
  - Pattern: `data JSONB` column with versioned enum contents (e.g., StagingEntryData::V1, TransactionData::V1)
  - Usage: Store error context during processing, void operation reasons, or other structured metadata
  - Nullable: JSONB data fields should be nullable to allow normal operations without context data
  - Migration: Add column with `ALTER TABLE table_name ADD COLUMN data JSONB;`
- **Atomic Transactions**: Use `transaction_async` for operations spanning multiple tables
- **Date Types**: Use `time::PrimitiveDateTime` for TIMESTAMP columns (not `time::Date` for DATE columns anymore)
<!-- AI-UPDATED: 2025-01-21 - Entity ID column naming convention -->
- **Entity ID Naming**: ID columns that identify entities should use descriptive names instead of generic `id`
  - Pattern: `{entity_name}_id` (e.g., `transformation_id`, `recon_rule_source_id`)
  - Exception: Foreign key columns and the `transactions` table (which already has a `transaction_id` column)
  - For Diesel models with Identifiable trait, requires `#[diesel(primary_key(column_name))]` attribute
  - Improves code clarity and prevents ambiguity when joining tables
<!-- AI-UPDATED: 2025-01-07 - Custom serde for PrimitiveDateTime -->
- **DateTime Serialization**: API models use `#[serde(with = "common_utils::custom_serde::iso8601")]` for consistent ISO8601 format
- **Optional DateTime Fields**: Use `#[serde(with = "common_utils::custom_serde::iso8601::option")]` for Option<PrimitiveDateTime>
- **UTC Storage**: All timestamps stored and retrieved in UTC, no timezone information preserved
- **Profile timezone**: lives in `business_profile.timezone`, never in timestamp columns; intentionally no SQL constraint (see `ai-docs/patterns/HANDLE-TIMEZONE.md`)
<!-- AI-UPDATED: 2025-07-16 - Version field for optimistic locking -->
- **Version Field Pattern**: All entities with update operations should include `version: i32` field
  - Default value 1 for new records
  - Must be incremented on every update for optimistic locking
  - Index should be created on version field for query performance
  - Required in all New, Update, and main model structs
<!-- AI-UPDATED: 2025-07-22 - Added transformation tracking fields to staging_entries -->
- **Transformation Tracking Pattern**: Staging entries track their transformation origin
  - `transformation_id`: NOT NULL VARCHAR(64) - identifies the transformation config used
  - `transformation_history_id`: NULLABLE VARCHAR(64) - identifies specific execution instance
  - Both fields indexed for query performance
  - Direct API entries use NULL for transformation_history_id
  - Migration: `2025-07-21-145742_add_transformation_id_to_staging_entries`
<!-- AI-UPDATED: 2025-08-11 - search_duplicates implementation with encryption and hashing -->
- **search_duplicates Design**: Dual-storage approach with encrypted and hashed field values
  - **Fields**: search_duplicate_id, staging_entry_id, account_id, merchant_id, transformation_id, field_path, encrypted_field_value (BYTEA), hashed_field_value (VARCHAR(255) with Secret<String> masking), timestamps
  - **No DB-level unique constraint**: the `(transformation_id, field_path, hashed_field_value)` unique index was dropped in `2025-09-21-080521_remove_search_duplicates_unique_constraint` (to allow multiple duplicate entries to be recorded); uniqueness is now checked at the application layer (`process_duplicate::check_for_duplicate`) via a lookup query, not a constraint
  - **Security**: Raw values encrypted using merchant_id scope, hashes used for duplicate detection with Secret masking
  - **Encryption Context**: Uses `Identifier::ReconMerchant(merchant_id)` for proper encryption scoping
  - **Masking**: hashed_field_value uses `masking::Secret<String>` to prevent accidental logging/exposure
  - Multiple records per staging_entry_id allowed for different field_path values
  - <!-- AI-UPDATED: 2026-08-10 - Composite unique constraints added --> **field_path is `FieldPathKey`, not `EntryField`**: composite constraints (`UniqueConstraintTypeV1::CompositeField`) have no single `EntryField` to store. `FieldPathKey` (`common_types::field_path_key`) is a generic opaque-string key, also used by `search_expectations.target_key` — not nested under any one domain since both need it. Column stays `TEXT`, no migration needed.
<!-- AI-UPDATED: 2025-08-13 - Staging entries metadata encryption schema changes -->
- **Staging Entries Encryption Schema**: Major changes for metadata encryption implementation
  - **merchant_id Column**: Added NOT NULL VARCHAR(64) for encryption key isolation
  - **metadata Column Type**: Changed from JSONB to BYTEA for encrypted storage
  - **Migration Pattern**: Used `convert_to(metadata::text, 'UTF8')` for safe type conversion
  - **Critical**: All staging entries now require merchant_id for proper encryption scoping
  - **Storage Format**: Encrypted metadata stored as `v1:base64` format regardless of encryption strategy
<!-- AI-UPDATED: 2025-01-21 - Status transition tracking pattern for updates -->
- **Status Transition Tracking Pattern**: Track original status when updating entities to voided/discarded state
  - Pattern: Use existing `discarded_status` field to preserve original status before void operation
  - Implementation: Before setting new status, store current status in `discarded_status` field  
  - Purpose: Audit trail showing what status entity had before being voided/discarded
  - Example: Transaction with status="Posted" voided stores "Posted" in discarded_status before setting status="Void"
<!-- AI-UPDATED: 2025-08-13 - Entries metadata encryption schema changes -->
- **Entries Metadata Encryption Schema**: Parallel implementation following staging_entries pattern
  - **merchant_id Column**: Added NOT NULL VARCHAR(64) for encryption key isolation
  - **metadata Column Type**: Changed from JSONB to BYTEA for encrypted storage
  - **Migration Pattern**: Exact same approach as staging_entries for consistency
  - **Schema Consistency**: Both entries and staging_entries now use identical encryption patterns
  - **Domain Model Support**: Diesel models use Encryption type, Domain models use EncryptableValue (metadata now mandatory)
<!-- AI-UPDATED: 2025-10-28 - Added order_id column to staging_entries and entries -->
- **order_id Column Addition**: New main field for order/transaction identifier tracking
  - **Column Type**: VARCHAR(64) NOT NULL for both staging_entries and entries tables
  - **Migration Strategy**: Used DEFAULT '' temporarily to handle existing data, then dropped default
  - **Indexes**: Created on order_id column in both tables for query performance
  - **Purpose**: Stores real-world order/transaction identifiers separate from internal transaction entity ID
  - **Validation**: MaxLength(64) enforced at metadata schema validation level to prevent database errors
  - **Migration**: `2025-10-28-083242_add_order_id_to_staging_entries_and_entries`
- **`api_key` table**: stores only a keyed hash, soft deleted via `status` (`revoked`) and `last_modified_at`; see `domains/api-keys/AGENT-NOTES.md`
- **Query Method Naming**: Use `list_by_*` for Vec returns, `find_by_*` for single/Option returns
  - Pattern: `list_by_staging_entry_id() -> Vec<T>` not `find_by_staging_entry_id() -> Vec<T>`
  - Follows existing convention in account.rs, transformation_config.rs, staging_entry.rs
- **Error Handling Patterns**: Keep query error handling simple and consistent
  - List operations: Use direct `.change_context(errors::DatabaseError::Others)` 
  - Optional finds: Use `generics::generic_find_one_optional` instead of manual match statements
  - Avoid complex match patterns unless handling specific business cases (like account.rs NotFound)
<!-- AI-UPDATED: 2025-09-04 - Staging entry versioning database pattern -->
- **Staging Entry Versioning Pattern**: Two-ID system with primary key versioning
  - **Primary Key**: `id` VARCHAR(64) - unique across all versions for database operations
  - **Logical ID**: `staging_entry_id` VARCHAR(64) - groups versions of same logical entity
  - **Version Field**: `version` INT4 NOT NULL DEFAULT 1 - incremented for each new version
  - **Archive Fields**: `discarded_at`, `discarded_status`, `discarded_data` for soft delete pattern
  - **Migration**: Comprehensive schema change converting single-row to multi-version pattern
  - **Index Strategy**: Primary key on `id`, composite index on `(staging_entry_id, version)` for version queries
  - **Query Pattern**: Use `staging_entry_id` + `discarded_at IS NULL` to get latest active version

## Design Decisions
- **Why Soft Deletes**: 
  - Audit trail requirements
  - Recovery capabilities
  - Historical balance accuracy
- **Why Separate Insert/Update Structs**:
  - Type safety for required vs optional fields
  - Prevents accidental field updates
  - Clear API contracts
- **Why Profile-Scoped Queries**:
  - Multi-tenant data isolation
  - Security by default
  - Performance optimization

## Relationships
- **Foreign Keys**: Not enforced at DB level - validated in application
- **Indexes**: Created on common query patterns (profile_id, account_id)
- **Unique Constraints**: Minimal - most uniqueness is scoped (e.g., within profile)

## When Working Here
- **New Migrations**:
  - Create enums before tables
  - Add indexes for foreign key fields
  - Include down.sql for rollback
- **New Models**:
  - Create three structs: Model, New, Update
  - Implement query methods in diesel_models
  - Add interface trait in db module
- **Query Patterns**:
  - Always scope by profile_id when applicable
  - Use `to_not_found_response` for 404 handling
  - Chain `.change_context()` for error conversion

<!-- AI-UPDATED: 2025-07-22 - Adding fields to existing tables pattern -->
- **Adding Fields to Existing Tables**:
  - Migration: Add column with NOT NULL constraint and index if needed
  - Diesel models: Update ALL structs (main model, New, Update if applicable)
  - API models: Update response DTOs to include new field
  - Conversion functions: Update all to_response() implementations
  - Core logic: Update constructor calls to pass new field values
  - Schema regeneration: Run `diesel migration run` to update schema.rs automatically
  <!-- AI-UPDATED: 2025-01-04 - Added batch query pattern -->
  - **Batch Query Pattern**: `list_all_staging_entries` bypasses profile scoping for system-wide operations
    - Optional status filter: `Some(DieselStagingEntryStatus::Pending)` — its only caller wants pending entries, which is a single flat status; a caller wanting "every manual-review reason" would need the `to_domain_statuses()` fan-out and an `eq_any` instead
    - Mandatory limit: `20` to control batch size (prevents unbounded queries)
    - Orders by `created_at ASC` to process oldest entries first (FIFO)
    - Security consideration: Only use for internal batch processing, not user-facing APIs
  <!-- AI-UPDATED: 2025-01-08 - Batch insert implementation -->
  - **Batch Insert Pattern**: Use `batch_insert()` methods for bulk operations
    - Empty batch check: Returns empty Vec without database call if input is empty
    - Single transaction: All entries inserted atomically or none at all
    - Performance optimization: Avoids N+1 query problems with large datasets
    - Error handling: Uses `get_results_async()` to return all inserted records
    - Logging: Includes entry count in debug logs for monitoring

## Migration Patterns
```sql
-- Enum creation (must come first)
CREATE TYPE "StatusEnum" AS ENUM ('value1', 'value2');

-- Table creation
CREATE TABLE table_name (
    id VARCHAR(64) PRIMARY KEY,
    -- Use BIGINT for amounts
    amount BIGINT NOT NULL,
    -- Use enum types directly
    status "StatusEnum" NOT NULL,
    -- Use TIMESTAMP for datetime fields
    effective_at TIMESTAMP NOT NULL,
    -- Version field for optimistic locking
    version INT4 NOT NULL DEFAULT 1,
    -- Soft delete fields
    discarded_at TIMESTAMP,
    discarded_status "StatusEnum",
    -- Timestamps
    created_at TIMESTAMP NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);

-- Indexes
CREATE INDEX idx_table_profile_id ON table_name(profile_id);
CREATE INDEX idx_table_version ON table_name(version);
```

-- Adding version column to existing table
```sql
-- Add version column with default value
ALTER TABLE existing_table ADD COLUMN version INT4 NOT NULL DEFAULT 1;

-- Create index for version field
CREATE INDEX idx_existing_table_version ON existing_table(version);
```

-- Adding metadata encryption to existing table
```sql
-- Add merchant_id column for encryption key isolation
ALTER TABLE table_name ADD COLUMN merchant_id VARCHAR(64) NOT NULL;

-- Convert metadata from JSONB to BYTEA for encrypted storage
ALTER TABLE table_name ALTER COLUMN metadata TYPE BYTEA 
USING convert_to(metadata::text, 'UTF8');
```

## Transaction Patterns
<!-- AI-UPDATED: 2024-06-24 - Added transaction pattern from recon-rules -->
```rust
// Multi-table atomic operations
async fn create_with_related_entities(
    &self,
    main_entity: MainEntityNew,
    related_entities: Vec<RelatedEntityNew>,
) -> StorageResult<(MainEntity, Vec<RelatedEntity>)> {
    use async_bb8_diesel::AsyncConnection;
    use diesel_models::errors::DatabaseError;
    use error_stack::ResultExt;

    let conn = self.get_connection().await?;

    conn.transaction_async(|connection_pool| async move {
        // Insert main entity first
        let created_main = main_entity
            .insert(&connection_pool)
            .await
            .map_err(|_| DatabaseError::Others)?;

        // Insert all related entities
        let mut created_related = Vec::new();
        for entity in related_entities {
            let created = entity
                .insert(&connection_pool)
                .await
                .map_err(|_| DatabaseError::Others)?;
            created_related.push(created);
        }

        Ok::<_, DatabaseError>((created_main, created_related))
    })
    .await
    .change_context(DatabaseError::DatabaseConnectionError)
    .attach_printable("Failed to create entities in transaction")
}
```

## Testing Gotchas
- **Test Database**: Each test runs in a transaction that's rolled back
- **Enum Values**: Can't be rolled back - test carefully
- **Connection Pool**: Test pool exhaustion scenarios
- **Concurrent Access**: Test optimistic locking patterns
- **Query Performance**: Add EXPLAIN tests for complex queries
- **Transaction Rollback**: Test that partial failures roll back entire transaction
- **JSON Serialization**: Test complex JSON types serialize/deserialize correctly in DB
