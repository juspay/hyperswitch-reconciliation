# OneToOne Strategies

**Account Mapping**: All OneToOne strategies map 1 source account → 1 target account  
**Entry Patterns**: Different cardinalities of source and target entries

## OneToOneSingleSingle

**Entry Pattern**: 1 source entry → 1 target entry  
**Use Case**: Simple one-to-one transaction matching  
**Strategy Type**: `OneToOneSingleSingleV1`

### Transaction Mode
1. Validate source and target accounts (same currency required)
2. Create direct entry from staging entry
3. Create ONE expected entry for target account with opposite balance direction
4. Create search expectation linking them

### Confirmation Mode
1. Find expected entry via search expectation
2. Validate match rules between staging entry and expected entry
3. Archive old transaction, create new transaction with updated status

### Status Logic
- **Match Failed** → `Mismatched`
- **Match Passed + Remaining Expected** → `Expected`
- **Match Passed + No Remaining** → `Posted`

<!-- AI-UPDATED: 2026-03-06 - Added tolerance behavior for OneToOneSingleSingle -->
### Tolerance Behavior (SingleSingle only)
- `target_account.tolerance_config` is optional and versioned (`ToleranceConfigV1`).
- `lower_tolerance` applies to under-amount scenarios, `upper_tolerance` applies to over-amount scenarios.
- When amount delta is within tolerance, reconciliation creates a balancing entry against `tolerance_account_id`.
- Exact boundary deltas (`delta == tolerance`) are treated as within tolerance.
- Tolerance support is limited to `OneToOneSingleSingle`; other OneToOne variants do not use tolerance config.

## OneToOneSingleMany

**Entry Pattern**: 1 source entry → many target entries  
**Use Case**: Split transactions (payment split into principal and fees)  
**Strategy Type**: `OneToOneSingleManyV1`

### Key Difference from SingleSingle
- **SingleSingle**: 1 source → 1 expected entry (1:1 match)
- **SingleMany**: 1 source → 1 expected entry, MULTIPLE target stagings reduce it (1:1:many)

### Transaction Mode
Creates ONE expected entry with full amount (not multiple expected entries).

### Confirmation Mode
Each confirmation **reduces the single expected entry's amount**:

```rust
let remaining_amount = expected_amount - confirmed_amount;
if staging_entry_amount <= remaining_amount {
    if staging_entry_amount == remaining_amount {
        // Full fulfillment
    } else {
        // Partial - create new expectation for remaining
    }
} else {
    // Overpayment → Mismatched
}
```

## OneToOneManySingle

**Entry Pattern**: Many source entries → 1 target entry  
**Use Case**: Aggregate transactions (daily batch settlement)  
**Strategy Type**: `OneToOneManySingleV1`

### Grouping Field
```rust
pub struct OneToOneManySingleSource {
    pub grouping_field: EntryField,  // Groups multiple source entries
}
```

### Transaction Mode
1. Group multiple source entries by `grouping_field` value
2. Aggregate amounts from all grouped entries
3. Create ONE expected entry for target with aggregated amount
4. Create search expectations (one per source entry → same expected entry)

### Confirmation Mode
Validates staging entry belongs to expected group, then processes.

### Source Accumulation
`source_accumulation_mode` controls how later sources evolve an open group:

- `versioned` rebuilds the transaction as a new version and is the default.
- `incremental_while_expected` appends to the current version only until a target arrives.

## OneToOneManyMany

**Entry Pattern**: Many source entries → many target entries  
**Use Case**: Complex batch processing with multiple outputs  
**Strategy Type**: `OneToOneManyManyV1`

### Key Features
- **Grouping Field**: Like ManySingle, groups related source entries
- **Search Groups**: Links all related entries for aggregate tracking
- **Single Expected Entry**: Despite "many to many", creates ONE expected entry
- **Multiple Confirmations**: Many target stagings reduce the single expected entry

### Transaction Mode
1. Create/find search group
2. Process multiple source entries (same grouping value)
3. Create ONE expected entry with aggregate amount
4. Use `get_remaining_entry_direction_and_amount` for balance calculation

### Confirmation Mode
1. Find search group for related entries
2. Aggregate all entries from both accounts
3. Each confirmation reduces the single expected entry's amount
4. Validate aggregate balances and metadata

### Status Logic (Complex)
- **No Target Entries** → `Expected`
- **Balance Mismatch + Same Direction** → `UnderAmount`
- **Balance Mismatch + Different Direction** → `OverAmount`
- **Metadata Mismatch** → `DataMismatch`
- **All Match** → `Posted(Reconciled)`

### Balance Calculation
```rust
pub fn get_remaining_entry_direction_and_amount(
    remaining_entries: &[DomainEntry],
    staging_entry: &DomainStagingEntry,
) -> (BalanceDirection, MinorUnit)
```

Sums all credit/debit entries and returns the net direction and absolute difference.

### EntryCreationData Pattern
```rust
pub struct EntryCreationData {
    pub entry_id: String,
    pub account_id: String,
    pub amount: MinorUnit,
    pub entry_type: BalanceDirection,
    // ... other fields
}
```

Intermediary type for clean entry manipulation between domain and transaction formats.

### Context Types
```rust
pub enum ManyToManyTransactionContext {
    NewSearchGroup(Box<...>),
    ExistingSearchGroup(Box<...>),
}
```

Handles scenarios based on search group existence.
