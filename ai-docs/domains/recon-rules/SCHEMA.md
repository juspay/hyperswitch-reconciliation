# Recon Rule Schema Reference

## Schema Location & Navigation

All recon rule schemas are defined as Rust types in:
```
crates/common_types/src/recon_rule/
```

### Root Schema
- **File**: `crates/common_types/src/recon_rule.rs`
- **Type**: `ReconStrategy` enum → `ReconStrategyV1` enum
- **JSON Tag**: `recon_strategy_version` and `recon_strategy_type`

### Strategy Types
- **OneToOne**: `crates/common_types/src/recon_rule/strategies/one_to_one_strategy.rs`
- **OneToMany**: `crates/common_types/src/recon_rule/strategies/one_to_many_strategy.rs`

### Component Schemas
- **EntryField**: `crates/common_types/src/recon_rule/entry_field.rs` - Field reference enum
- **MatchRules**: `crates/common_types/src/recon_rule/match_rules.rs` - Field comparison rules
- **SearchIdentifier**: `crates/common_types/src/recon_rule/search_identifier.rs` - Source/target field mapping
- **Trigger**: `crates/common_types/src/recon_rule/trigger.rs` - Rule activation conditions
- **LinkWithRule**: `crates/common_types/src/recon_rule/link_with_rule.rs` - Rule chaining (OneToMany)

## Schema Structure Overview

```
ReconStrategy (versioned enum)
└── ReconStrategyV1 (tagged by recon_strategy_type)
    ├── OneToOne(OneToOneStrategy)
    │   └── OneToOneStrategyV1 (tagged by one_to_one_strategy_type)
    │       ├── SingleSingle (1 source entry → 1 target entry)
    │       ├── SingleMany (1 source entry → many target entries)
    │       ├── ManySingle (many source entries → 1 target entry)
    │       └── ManyMany (many source entries → many target entries)
    └── OneToMany(OneToManyStrategy)
        └── OneToManyStrategyV1 (tagged by one_to_many_strategy_type)
            └── SingleSingle (1 source entry → 1 entry per target account)
```

## Critical Schema Gotchas

### Flattened JSON Structure
**All strategy fields are flattened at the same level** - not nested under strategy type:

```json
{
  "recon_strategy_version": "v1",
  "recon_strategy_type": "one_to_one",
  "one_to_one_strategy_version": "v1",
  "one_to_one_strategy_type": "single_single",
  "one_to_one_single_single_version": "v1",
  // All fields below are at SAME level (not nested)
  "search_identifier": {...},
  "match_rules": {...},
  "source_account": {...},
  "target_account": {...}
}
```

This is enforced by serde's `#[serde(flatten)]` attribute on strategy variants.

### Versioning Pattern
Every schema level has its own version field:
- Root: `recon_strategy_version`
- Strategy type: `one_to_one_strategy_version` or `one_to_many_strategy_version`
- Specific variant: `one_to_one_single_single_version`, `one_to_one_single_many_version`, etc.

### EntryField Format
- Standard fields: `amount`, `currency`, `entry_type`, `effective_at`, `order_id`
- Metadata fields: `metadata.key` (nested paths supported: `metadata.payment.method`)

### Trigger Versions
- `Trigger::V1` is the legacy single-condition shape:

```json
{
  "trigger_version": "v1",
  "field": "metadata.transaction_id",
  "operator": { "operator_version": "v1", "value": "equals" },
  "value": "test_value"
}
```

- `Trigger::V2` adds flat composite boolean logic across multiple conditions:

```json
{
  "trigger_version": "v2",
  "logic": "all",
  "conditions": [
    {
      "field": "metadata.payment_type",
      "operator": { "operator_version": "v1", "value": "equals" },
      "value": "card"
    },
    {
      "field": "currency",
      "operator": { "operator_version": "v1", "value": "equals" },
      "value": "USD"
    }
  ]
}
```

#### Trigger V2 Constraints
- `logic` supports only `all` and `any`
- `conditions` must contain at least one entry
- `TriggerCondition` reuses the same `field` / `operator` / `value` shape as V1
- Nested boolean groups are **not** supported in V2; `(A AND B) OR C` needs a future trigger version
- Missing field values still evaluate to `false`

### Grouping Field (ManySingle & ManyMany only)
Located in `source_account.grouping_field` - groups multiple source entries for aggregation.

### Source Accumulation Mode (ManySingle only)
`source_accumulation_mode` controls how a source entry joining an existing group evolves its transaction:

- `versioned` (default) — rebuild the transaction as a new version.
- `incremental_while_expected` — append the source to the current version while the transaction is still waiting for its target; all other states use normal versioning.

### Completion Policy (ManySingle only)
<!-- AI-UPDATED: 2026-10-18 - Optional closure on target arrival -->
`completion_policy` is independent of `source_accumulation_mode` and defaults to `keep_open` for omitted request fields and existing stored rules:

- `keep_open` preserves source accumulation after target arrival, except for currency mismatches.
- `close_on_target` stops source accumulation once a target is successfully incorporated, regardless of the match outcome. Amount differences become `UnderAmount(Mismatch)` / `OverAmount(Mismatch)` rather than `Expected`.

Both modes keep the search group and expectation while waiting for a target. Closure archives the old search artifacts without replacements; a later source with the same grouping value starts a new transaction.

Rule updates apply on the next processing event, without backfilling existing groups. A source joining a previously open group that already has a target is included before closure. Switching back to `keep_open` does not reopen archived groups.

### OneToMany Split Logic
`target_accounts` contains split configuration:
- **Percentage split**: Each target gets % of source amount (must sum to 1.0)
- **Fixed split**: Each target gets fixed amount or "remaining" for balance

### Database Storage
- **Table**: `recon_rules`
- **Column**: `strategy` (JSONB type)
- **Format**: Contains entire `ReconStrategy` JSON

## Common Operations

### Finding Schema Examples
Look in test files:
```
crates/common_types/src/recon_rule/strategies/
  - Tests in one_to_one_strategy.rs
  - Tests in one_to_many_strategy.rs
```

### Validation Logic
Search for `impl Validate` or validation functions in the schema files to understand validation rules.

For triggers specifically:
- runtime evaluation lives in `crates/recon/src/domain/recon_engine/implementations/trigger.rs`
- create-time trigger validation lives in `crates/recon/src/utils/recon_rule.rs`
- V2 empty-condition payloads are rejected during rule creation instead of being stored and failing later

### API Layer
- **V2 API**: Uses `ReconStrategy` directly (flattened JSON)
- **Request body**: `strategy` field contains full schema
- **Response**: Same flattened structure
