# OneToMany Strategies

**Account Mapping**: All OneToMany strategies map 1 source account → multiple target accounts  
**Entry Patterns**: Different patterns for distributing entries to targets

## OneToManySingleSingle Strategy

### Overview
**Account Pattern**: 1 source account → multiple target accounts  
**Entry Pattern**: 1 source entry → 1 entry per target account  
**Use Case**: Splitting a single payment to multiple recipients (e.g., marketplace payouts)  
**Strategy Type**: `OneToManySingleSingleV1`

### Key Features

#### Split Configuration
```rust
pub enum OneToManySingleSingleTargetType {
    Percentage {
        targets: Vec<(OneToManySingleSingleTarget, PercentageSplitValue)>,
    },
    Fixed {
        targets: Vec<(OneToManySingleSingleTarget, FixedSplitValue)>,
    },
}
```

#### Split Types
1. **Percentage Split**: Distribute by percentage (must sum to 1.0)
2. **Fixed Split**: Distribute fixed amounts with optional "remaining" for balance

#### Target Configuration
- **Each Target Has**: Own account_id, search_identifier, and match_rules
- **Multiple Expected Entries**: Creates ONE expected entry per target account
- **Independent Processing**: Each target processed as separate expectation
- **Atomic Execution**: All target expected entries created in single transaction

### Transaction Mode Flow
1. **Validate Split**: Ensure percentages sum to 1.0 or fixed amounts are valid
2. **Calculate Amounts**: Determine amount for each target based on split type
3. **Create Expected Entries**: ONE expected entry per target account (multiple expected entries total)
4. **Create Expectations**: Independent search expectation for each target's expected entry

### Confirmation Mode Flow
1. **Find Target**: Match staging entry to specific target's expected entry
2. **Validate Match**: Apply target-specific match rules
3. **Update Status**: Mark specific target's expected entry as confirmed
4. **Track Progress**: Monitor which targets' expected entries have been confirmed
