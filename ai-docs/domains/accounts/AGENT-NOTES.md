# Accounts - Agent Notes

<!-- AI-UPDATED: 2025-07-08 - Account balance storage refactored from stored balances to debit/credit components -->

## Critical Rules
- **Balance Fields**: Accounts store debit/credit components separately:
  - `initial_balance`: The starting balance when account was created (immutable)
  - `posted_debits` and `posted_credits`: Accumulated debit and credit amounts from posted entries
  - `pending_debits` and `pending_credits`: Accumulated debit and credit amounts from pending entries
- **Dynamic Balance Calculation**: Balances are calculated dynamically from debit/credit components:
  - `posted_balance` = `initial_balance` + (`posted_credits` - `posted_debits`)
  - `pending_balance` = `initial_balance` + (`pending_credits` - `pending_debits`)
- **BalanceDirection**: Account type determines how balances behave:
  - `Credit` accounts: Positive balance means money owed TO you
  - `Debit` accounts: Positive balance means money owed BY you
- **Currency Immutability**: An account's currency CANNOT be changed after creation
- **Profile Isolation**: Accounts are scoped to business profiles - always validate profile_id in queries

## Common Gotchas
- **Balance Updates**: Balance updates now happen through entry processing - debit/credit components are updated when entries are created or change status
- **No Direct Balance Manipulation**: The `update_balances_by_account_id` method has been removed - all balance changes MUST come through the entry system
- **MinorUnit Storage**: All amounts are stored as MinorUnit (cents/paise) in database as BIGINT to avoid floating point issues
- **Account Name Updates Only**: The only mutable field via AccountUpdate is account_name - everything else requires new account creation
- **Profile-Account Uniqueness**: An account_id must be unique within a profile_id, not globally
- **Balance Calculation**: Always calculate balances dynamically from debit/credit components - never store calculated balances

## Design Decisions
- **Why Separate Debit/Credit Storage**: 
  - Enables true double-entry bookkeeping at the storage level
  - Provides complete audit trail of all debits and credits
  - Balances are calculated, not stored, ensuring consistency
  - Eliminates possibility of balance drift from calculation errors
- **Why Dynamic Balance Calculation**:
  - Ensures balances always reflect actual entry data
  - Prevents inconsistencies between stored balances and entry totals
  - Simplifies reconciliation by having single source of truth
- **Why BalanceDirection Enum**: 
  - Type-safe representation of accounting nature
  - Prevents accidental sign inversions in calculations
  - Makes double-entry bookkeeping explicit
- **Why No Direct Balance Updates**:
  - All balance changes must come through entries for audit trail
  - Ensures proper double-entry bookkeeping is maintained
  - Prevents manual balance manipulation that could break consistency

## Relationships
- **Entries**: One-to-many relationship - entries affect account balances
- **Staging Entries**: One-to-many relationship - staging entries become entries
- **Business Profile**: Many-to-one - accounts belong to exactly one profile
- **Transactions**: Indirect relationship through entries

## Balance Updates Through Entries
<!-- AI-UPDATED: 2025-07-30 - Enhanced balance calculation with status-specific separation -->
- **Entry-Based Updates**: All balance changes now happen through the entry system
- **Status-Specific Balance Updates**: Each entry status affects specific balance fields:
  - `Pending` entries: Update `pending_debits`/`pending_credits` only
  - `Posted` entries: Update `posted_debits`/`posted_credits` only
  - `Expected` entries: Update `expected_debits`/`expected_credits` only
  - `Mismatched` entries: Update `mismatched_debits`/`mismatched_credits` only
  - `Archived` entries: No balance impact (all deltas are zero)
- **Breaking Change from Old Behavior**: Previously, Posted entries affected both pending and posted balances
  - Old: Posted entries increased both pending and posted balance fields
  - New: Posted entries only affect posted balance fields
  - This enables finer balance granularity for reconciliation
- **Balance Reversal on Archive**: When entries are archived (discarded):
  - Each entry status reversal only affects its specific balance fields
  - Status-to-balance mapping is consistent between creation and reversal
  - Already archived entries: No impact (function returns zeros)
  - Only reverses if entry status actually changed to prevent double-reversal
- **No Manual Adjustments**: Direct balance manipulation has been removed
- **Audit Trail**: Every balance change is traceable to specific entries and statuses
- **Database Migration Required**: New balance fields require migration to separate existing entries by status


## When Working Here
- **Creating Accounts**: Always validate:
  - Profile exists and belongs to authenticated merchant
  - Currency is valid and supported
  - Initial balance makes sense for account type
- **Querying Accounts**: Always include profile_id in queries for security
- **Balance Checks**: Never trust client-provided balances - always recalculate
- **Updates**: Only account_name can be updated - for other changes, create new account
- **Batch Account Validation**: Use `find_accounts_by_profile_id_and_account_ids` for validating multiple accounts:
  - Optimized single query with IN clause instead of multiple individual lookups
  - Returns only accounts that exist AND belong to the specified profile
  - Returned vector may be shorter than input if some accounts don't exist or belong to different profile
  - Order of returned accounts not guaranteed to match input order
  - Automatically handles deduplication of account IDs
  - Preferred method for recon rule validation and similar batch operations

## API Behavior
<!-- AI-UPDATED: 2025-07-30 - Enhanced API response with new balance fields -->
- **List Accounts**: Returns all accounts for a profile with dynamically calculated balances
- **Get Account**: Requires both account_id AND profile_id for security
- **Create Account**: Sets initial_balance and all 8 debit/credit components to zero
- **Update Account**: Only accepts account_name changes
- **Enhanced Balance Response**: API returns 4 calculated balance types:
  - `pending_balance`: From pending_debits/pending_credits
  - `posted_balance`: From posted_debits/posted_credits  
  - `expected_balance`: From expected_debits/expected_credits
  - `mismatched_balance`: From mismatched_debits/mismatched_credits
  - All balances calculated dynamically, not stored directly
<!-- AI-UPDATED: 2025-08-20 - Added debit/credit breakdown in account responses -->
- **Debit/Credit Breakdown**: API now returns individual debit/credit components for each status:
  - `posted_debits`, `posted_credits` - Raw posted amounts
  - `pending_debits`, `pending_credits` - Raw pending amounts  
  - `expected_debits`, `expected_credits` - Raw expected amounts
  - `mismatched_debits`, `mismatched_credits` - Raw mismatched amounts
  - Enables detailed audit trails and balance calculation transparency
  - All amounts returned as AmountResponse objects with value and currency
- **File Upload**: Account-scoped file uploads via `POST /accounts/{account_id}/upload_file`
<!-- AI-UPDATED: 2025-07-16 - Flattened account data in transaction responses -->
- **Flattened Account Data**: New `AccountData` struct provides complete account information for embedded responses
  - Used in `EntryData` and `TransactionWithEntriesResponse` for single-query efficiency
  - `EntryData` now includes `status` and `amount` fields alongside embedded account data
  - Includes all account fields with proper datetime serialization
  - Eliminates need for separate account lookups in transaction/entry queries
<!-- AI-UPDATED: 2025-01-10 - Entry status filtering -->
- **List Account Entries**: Supports filtering by status via query parameter `?status=pending,posted`
  - Query struct `ListEntriesQuery` defined in `api_models::entry` module
  - Uses custom serde deserializer `parse_comma_separated` for parsing comma-separated values
  - Supports multiple status values for flexible filtering
  - Database query uses `eq_any` for efficient filtering
  - Empty filter returns all entries (backward compatible)

## File Upload Gotchas
<!-- AI-UPDATED: 2025-01-07 - Implemented account-scoped file uploads -->
- **Framework-level validation**: Use `#[multipart(limit = "10MB")]` instead of manual size checks in core
  - Framework rejects oversized files before they reach business logic
  - Eliminates redundant validation in utils and core layers
- **Account-scoped storage path**: Files stored as `{merchant_id}/{profile_id}/{account_id}_{timestamp}/{filename}`
  - Timestamp ensures chronological organization within account
  - Account ownership validated before upload (must belong to authenticated profile)
- **FileType enum pattern**: Uses `strum::Display` like BalanceDirection for automatic string conversion
  - No manual match statements needed for enum-to-string conversion
  - Consistent with other enums in the codebase
- **Validation flow**: AccountFileUploadRequest → ProcessedFileRequest → Core processing
  - Framework validates size/format, utils convert types, core handles business logic
  - Each layer has distinct responsibilities, no overlap

## File Upload Architecture
<!-- AI-UPDATED: 2026-09-10 - Superseded: uploads moved to the ingestion domain -->
- **Uploads are not an account concern**: files are uploaded against an ingestion
  (`POST /ingestions/{ingestion_id}/upload`), not against an account. See `domains/ingestion/`.
- Account modules handle only generic account operations (CRUD, balance management).

## Testing Gotchas
<!-- AI-UPDATED: 2025-07-30 - Updated for status-specific balance behavior -->
- **Status-Specific Balance Assertions**: Test that each entry status only affects its corresponding balance fields
  - Pending entries should only change pending_debits/pending_credits
  - Posted entries should only change posted_debits/posted_credits
  - Expected entries should only change expected_debits/expected_credits
  - Mismatched entries should only change mismatched_debits/mismatched_credits
- **Breaking Change in Tests**: Old tests expecting Posted entries to affect pending balances will fail
  - Update test expectations to match new status-specific behavior
  - BalanceDelta struct now has 8 fields instead of 4
- **Component Testing**: Verify all 8 debit/credit components update correctly with entry changes
- **Currency Handling**: Test with different decimal place currencies (JPY vs USD)
- **Negative Balances**: Valid for some account types - don't assume positive only
- **Concurrent Updates**: Test race conditions on all 8 debit/credit component updates through entries
- **Balance Calculation**: Always verify: balance = initial + (credits - debits) for each balance type
- **Migration Testing**: Ensure existing entries are properly categorized by status during migration
