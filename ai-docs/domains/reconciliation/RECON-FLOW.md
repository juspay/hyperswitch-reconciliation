# Reconciliation Flow Lifecycle

## Processing Overview

The reconciliation engine processes staging entries through a deterministic flow that transforms input data into reconciled transactions. The flow is divided into two primary modes based on the staging entry's `processing_mode`.

## Entry Point: `process_staging_entry_core_v2`

```rust
async fn process_staging_entry_core_v2(
    state: &AppState,
    staging_entry: DieselStagingEntry,
    check_duplicate_exists: bool,
) -> ReconEngineResult<ReconTriggerResponse>
```

### Phase 1: Pre-Processing

#### 1.1 Decryption and Validation
- **Purpose**: Convert encrypted staging entry to domain model
- **Steps**:
  1. Get merchant-specific encryptor
  2. Decrypt staging entry metadata
  3. Validate transformation config exists
  4. Validate metadata schema exists

#### 1.2 Duplicate Detection (Optional)
- **Trigger**: `check_duplicate_exists = true`
- **Process**: Hash-based duplicate detection using transformation config
- **Outcome**: Duplicate entries marked for manual review

### Phase 2: Rule Orchestration

#### 2.1 Processing Decision Logic
```rust
match staging_entry.processing_mode {
    StagingEntryProcessingMode::Transaction => ProcessAsTransaction,
    StagingEntryProcessingMode::Confirmation => ProcessAsConfirmation,
}
```

#### 2.2 Rule Discovery
- **Transaction Mode**: Find rules where account is `source`
- **Confirmation Mode**: Find rules where account is `target`
- **Method**: Query `recon_rule_account` table by `(account_id, account_type)`
- **Validation**: All accounts must belong to authenticated profile

#### 2.3 Rule Filtering and Selection
- **Trigger Evaluation**: Apply rule triggers to staging entry
- **Priority Selection**: Choose highest priority rule among matches
- **No Match**: Entry marked for manual review

### Phase 3: Strategy Execution

## Transaction Mode Flow

### Overview
Creates new transactions with expected entries awaiting confirmation.

### Step-by-Step Process

#### 3.1 Strategy Preparation
```rust
let prep_data = strategy.get_recon_engine_type().prep_transaction(
    state,
    domain_staging_entry,
    transaction_context,
);
```

#### 3.2 Transaction Context Creation
- **Input**: Prepared data from strategy
- **Output**: Transaction context with all necessary data
- **Validation**: Currency consistency across all accounts

#### 3.3 Transaction Processing
```rust
let transaction = transaction_context.process_transaction_context(state);
```

#### 3.4 Atomic Operations
1. **Create Main Entry**: Direct entry from staging entry
2. **Create Expected Entries**: One per rule target with opposite balance direction
3. **Create Search Expectations**: Link source to expected targets
4. **Update Staging Entry**: Status changed to `Processed`
5. **Update Account Balances**: Reflect new pending amounts

#### 3.5 Success Response
```rust
ReconTriggerResponse::Success(Box::new(recon_transaction))
```

## Confirmation Mode Flow

### Overview
Fulfills existing search expectations and evolves transactions.

### Step-by-Step Process

#### 3.1 Multi-Target Search
- **Build Search Criteria**: For all applicable rules
- **Batch Query**: `find_for_multiple_targets()` with composite search
- **Result**: First matching expectation across all targets

#### 3.2 Strategy Preparation
```rust
let prep_data = strategy.get_recon_engine_type().prep_confirmation(
    state,
    domain_staging_entry,
    confirmation_context,
);
```

#### 3.3 Confirmation Context Creation
- **Input**: Prepared data and found expectation
- **Output**: Context with match validation data

#### 3.4 Confirmation Processing
```rust
let transaction = confirmation_context.process_confirmation_context(state);
```

#### 3.5 Transaction Evolution Pattern

#### Archive Phase
1. **Archive Old Transaction**: Status `Archived`
2. **Archive Old Entries**: Status `Archived`
3. **Archive Old Expectations**: Status `Archived`
4. **Update Staging Entry**: Status `Processed`

#### Creation Phase
1. **Create New Transaction**: Incremented version number
2. **Create Remaining Expected Entries**: Unmatched expectations
3. **Create Confirmed Entry**: Current staging entry as confirmed
4. **Create New Search Expectations**: For remaining expected entries

#### 3.6 Status Determination Logic
```rust
match (match_result, has_remaining_expected) {
    (Failed, _) => TransactionStatus::Mismatched,
    (Passed, true) => TransactionStatus::Expected,
    (Passed, false) => TransactionStatus::Posted,
}
```

## Error Handling Flow

### Error Context Processing
```rust
async fn process_error_context(
    state: &AppState,
    error: Report<ProcessingError>,
    staging_entry: &DieselStagingEntry,
) -> ReconEngineResult<String>
```

### Error Response Pattern
1. **Archive Original**: Mark original staging entry as discarded
2. **Create Manual Review**: New version with `NeedsManualReview` status
3. **Preserve Error**: Error type stored in entry data
4. **Return Error**: Error details returned to caller

## Strategy-Specific Flows

### OneToOneSingleSingle Strategy
- **Account Pattern**: 1 source account → 1 target account
- **Entry Pattern**: 1 source entry → 1 target entry
- **Use Case**: Simple transaction matching
- **Flow**: Standard transaction or confirmation flow

### OneToOneSingleMany Strategy
- **Account Pattern**: 1 source account → 1 target account
- **Entry Pattern**: 1 source entry → many target entries
- **Use Case**: Split transactions (e.g., payment into principal + fees)
- **Transaction Mode**: Creates multiple expected entries for same target account
- **Confirmation Mode**: Partial fulfillment support

### OneToOneManySingle Strategy
- **Account Pattern**: 1 source account → 1 target account
- **Entry Pattern**: Many source entries → 1 target entry
- **Use Case**: Aggregate transactions (e.g., daily batch settlement)
- **Special Feature**: Grouping field for source aggregation
- **Amount Calculation**: Sum of source amounts

### OneToOneManyMany Strategy
- **Account Pattern**: 1 source account → 1 target account
- **Entry Pattern**: Many source entries → many target entries
- **Use Case**: Complex multi-entry reconciliation, batch processing
- **Transaction Mode**: Creates search groups and multiple expected entries
- **Confirmation Mode**: Complex balance validation across all entries
- **Special Features**: 
  - Grouping field for organizing source entries (like ManySingle)
  - Search group management for related entries
  - EntryCreationData intermediary for clean conversion
  - Shared balance calculation utility
  - Complex status determination with mismatch types
- **Amount Calculation**: Uses `get_remaining_entry_direction_and_amount` for accurate balance tracking

### OneToManySingleSingle Strategy
- **Account Pattern**: 1 source account → multiple target accounts
- **Entry Pattern**: 1 source entry → 1 entry per target account
- **Use Case**: Splitting payments to multiple recipients (marketplace payouts)
- **Transaction Mode**: Creates entries for each target account with split amounts
- **Confirmation Mode**: Individual confirmations for each target
- **Special Features**:
  - Percentage or fixed amount split configuration
  - Independent search identifiers per target
  - Target-specific match rules
  - Atomic creation of all target entries
- **Split Types**: Percentage (must sum to 1.0) or Fixed amounts with optional \"remaining\"
