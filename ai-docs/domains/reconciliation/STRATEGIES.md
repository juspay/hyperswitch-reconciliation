# Reconciliation Strategy Implementations

## Overview

Reconciliation strategies define the specific logic for matching and processing transactions between source and target accounts. Each strategy implements the `ReconEngine` trait and handles different cardinality patterns of source-target relationships.

## Documentation Structure

- **[ONE-TO-ONE-STRATEGIES.md](./ONE-TO-ONE-STRATEGIES.md)** - All OneToOne strategy variants
- **[ONE-TO-MANY-STRATEGIES.md](./ONE-TO-MANY-STRATEGIES.md)** - OneToMany variants

## Strategy Architecture

### Core Trait: `ReconEngine`

```rust
#[async_trait::async_trait]
pub trait ReconEngine {
    type PrepTypeTransaction: Send;
    type PrepTypeConfirmation: Send;
    type ContextTypeTransaction: Send;
    type ContextTypeConfirmation: Send;
    type StatusDeterminationInput: Send;
    type StatusDeterminationOutput: Send;

    async fn prep_transaction(
        self,
        state: &AppState,
        staging_entry: DomainStagingEntry,
        transaction_context: TransactionContext,
    ) -> ProcessingResult<ReconPrepTransaction>;

    async fn prep_confirmation(
        self,
        state: &AppState,
        staging_entry: DomainStagingEntry,
        confirmation_context: ConfirmationContext,
    ) -> ProcessingResult<ReconPrepConfirmation>;

    // ... other methods
}
```

### Strategy Registration

```rust
define_recon_flow! {
    // OneToOne strategies (1 source account → 1 target account)
    OneToOneSingleSingleV1,
    OneToOneSingleManyV1,
    OneToOneManySingleV1,
    OneToOneManyManyV1,
    // OneToMany strategies (1 source account → multiple target accounts)
    OneToManySingleSingleV1
}
```

## Common Strategy Components

### Search Criteria Generation

```rust
fn get_search_criteria(
    &self,
    data: &impl ExtractFromEntryField,
    rule_id: &str,
) -> ProcessingResult<Vec<SearchCriteria>>
```

**Purpose**: Generate search criteria for finding matching expectations in confirmation mode.

**Implementation**:
1. **Extract Source Value**: Get value from source field using search identifier
2. **Create Search Criteria**: Bundle rule_id, target field, and source value
3. **Return List**: Multiple criteria for complex strategies

### Trigger Evaluation

```rust
fn evaluate_trigger(&self, data: &impl ExtractFromEntryField) -> bool
```

**Purpose**: Determine if strategy should process given staging entry.

**Logic**:
1. **Extract Field Value**: Get trigger field value from staging entry
2. **Apply Operator**: Compare with trigger value using specified operator
3. **Return Result**: True if trigger conditions satisfied

### Metadata Transfer

```rust
fn extract_expected_entry_metadata_from_source(
    &self,
    staging_entry: &DomainStagingEntry,
) -> ProcessingResult<serde_json::Map<String, serde_json::Value>>
```

**Purpose**: Transfer metadata from source entry to expected entry using match rules.

**Process**:
1. **Iterate Match Rules**: For each rule in match rules
2. **Check Source Field**: If source field is metadata type
3. **Extract Value**: Get value from staging entry metadata
4. **Map to Target**: Map to target field name for expected entry

## Error Handling Patterns

### Common Errors
- **MissingSearchIdentifierValue**: Required field not found in entry
- **MissingMatchField**: Field required for matching not found
- **MissingGroupingField**: Grouping field not found (ManySingle strategy)
- **Currency Mismatch**: Source and target accounts have different currencies

### Strategy-Specific Errors
- **Amount Mismatch**: Staging amount doesn't match expected amount
- **Overpayment**: Staging amount exceeds remaining expected amount
- **Group Validation Failed**: Staging entry doesn't belong to expected group

### Error Recovery
- **Manual Review**: Set entry status to `NeedsManualReview`
- **Error Context**: Include detailed error information in entry data
- **Retry Capability**: Some errors can be corrected and retried
