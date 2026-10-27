# Reconciliation System Documentation

## Overview

The reconciliation system provides automated transaction matching and settlement processing through configurable rules and strategies. This documentation covers the complete system architecture, implementation details, and operational guidance.

## Quick Start

### Core Concepts
- **Staging Entries**: Input transactions awaiting processing
- **Recon Rules**: Configurable matching logic and strategies  
- **Transactions**: Atomic groups of related entries
- **Processing Modes**: Transaction (creates expectations) vs Confirmation (fulfills expectations)

### Basic Flow
1. Create staging entries with `processing_mode`
2. System discovers and applies matching rules
3. Creates transactions with expected entries (Transaction mode)
4. Confirms entries and evolves transactions (Confirmation mode)

## Documentation Structure

### Core Architecture
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Complete system architecture and design
- **[RECON-FLOW.md](./RECON-FLOW.md)** - Detailed reconciliation lifecycle and flow
- **[STRATEGIES.md](./STRATEGIES.md)** - Strategy implementations and patterns

### Rules and Configuration
- **[../recon-rules/SCHEMA.md](../recon-rules/SCHEMA.md)** - Complete recon_rule schema reference
- **[../recon-rules/AGENT-NOTES.md](../recon-rules/AGENT-NOTES.md)** - Implementation gotchas and critical rules

### Operational Guidance
- **[AGENT-NOTES.md](./AGENT-NOTES.md)** - Critical rules, gotchas, and operational procedures

## Key Components

### 1. Reconciliation Engine (`recon_engine`)
- **Location**: `crates/recon/src/domain/recon_engine/`
- **Purpose**: Core processing logic and strategy dispatch
- **Key Files**:
  - `mod.rs` - Main engine interface
  - `rule_orchestration.rs` - Rule discovery and selection
  - `flows/` - Strategy implementations

### 2. Rule System (`recon_rule`)
- **Location**: `crates/common_types/src/recon_rule/`
- **Purpose**: Schema definitions and strategy types
- **Key Files**:
  - `mod.rs` - Root schema types
  - `strategies/` - Strategy implementations
  - `trigger.rs`, `match_rules.rs`, `search_identifier.rs` - Component schemas

### 3. Database Layer
- **Location**: `crates/storage_impl/src/`
- **Purpose**: Data access and persistence
- **Key Files**:
  - `recon_rule.rs` - Rule CRUD operations
  - `recon_rule_account.rs` - Account-rule mapping
  - `search_expectation.rs` - Expectation management

## Strategy Types

### OneToOne Strategies
**Account Pattern**: 1 source account → 1 target account

#### OneToOneSingleSingle
- **Entry Pattern**: 1 source entry → 1 target entry
- **Use Case**: Simple transaction matching
- **Features**: Direct entry creation, expectation matching

#### OneToOneSingleMany  
- **Entry Pattern**: 1 source entry → many target entries
- **Use Case**: Split transactions (e.g., payment into principal + fees)
- **Features**: Partial fulfillment, amount tracking

#### OneToOneManySingle
- **Entry Pattern**: Many source entries → 1 target entry
- **Use Case**: Aggregate transactions (e.g., daily batch settlement)
- **Features**: Grouping field, amount aggregation

#### OneToOneManyMany
- **Entry Pattern**: Many source entries → many target entries
- **Use Case**: Complex multi-entry reconciliation (e.g., batch processing)
- **Features**: Grouping field, search groups, N:M matching

### OneToMany Strategies
**Account Pattern**: 1 source account → multiple target accounts

#### OneToManySingleSingle
- **Entry Pattern**: 1 source entry → 1 entry per target account
- **Use Case**: Splitting payments to multiple recipients (e.g., marketplace payouts)
- **Features**: Percentage or fixed amount splits, multiple target accounts

## Processing Modes

### Transaction Mode
- **Trigger**: `processing_mode: "transaction"`
- **Action**: Creates new transaction with expected entries
- **Result**: Search expectations awaiting confirmation

### Confirmation Mode
- **Trigger**: `processing_mode: "confirmation"`  
- **Action**: Fulfills expectations, evolves transaction
- **Result**: Updated transaction status and entries

## Error Handling

### Common Errors
- `NoRulesFound` - No applicable rules for staging entry
- `MissingSearchIdentifierValue` - Required field not found
- `StagingEntryCurrencyMismatch` - Currency validation failure
- `DuplicateEntry` - Duplicate transaction detected

### Error Recovery
- Entries set to `NeedsManualReview` status
- Detailed error context stored in entry data
- Manual correction and retry capability

## Getting Help

### Documentation Resources
- **Architecture**: System design and component interaction
- **Schema**: Complete rule configuration reference
- **Examples**: Concrete implementation scenarios
- **Agent Notes**: Critical gotchas and operational wisdom

### Code Navigation
- **Core Logic**: `crates/recon/src/domain/recon_engine/`
- **Rule Types**: `crates/common_types/src/recon_rule/`
- **Database**: `crates/storage_impl/src/`
- **API**: `crates/recon/src/routes/`

### Common Issues
- **Rule Not Matching**: Check trigger conditions and field values
- **Currency Mismatches**: Verify account currency consistency
- **Missing Fields**: Ensure required fields present in metadata
- **Performance**: Review rule complexity and query patterns
