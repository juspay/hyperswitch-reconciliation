# Reconciliation System Architecture

## Overview

The reconciliation system processes financial entries through a rule-based engine that matches source and target transactions using configurable strategies. The system supports versioned data, encrypted metadata, and atomic processing operations.

## Core Components

### 1. Staging Entries
- **Purpose**: Input buffer for external transaction data
- **Features**: Versioned, encrypted metadata, transformation tracking
- **Processing Modes**: `Transaction` (creates expectations) or `Confirmation` (fulfills expectations)
- **Status Flow**: `Pending` → `Processed` or `NeedsManualReview`

### 2. Recon Rules
- **Purpose**: Define how entries are matched and reconciled
- **Structure**: Versioned JSON schema with triggers, match rules, and search identifiers
- **Strategies**: 
  - `OneToOne`: 1 source account → 1 target account (SingleSingle, SingleMany, ManySingle, ManyMany)
  - `OneToMany`: 1 source account → multiple target accounts (SingleSingle)
- **Storage**: Complex types stored as JSONB with custom Diesel traits

### 3. Transactions
- **Purpose**: Atomic grouping of related entries
- **Features**: Versioned evolution, status tracking, balance management
- **Status Flow**: `Expected` → `Posted` or `Mismatched`
- **Entries**: Multiple entries per transaction with double-entry accounting

### 4. Search Expectations
- **Purpose**: Link source transactions to expected target transactions
- **Structure**: Maps (source_account, source_key, source_value) → (target_account, target_key, expected_value)
- **Lifecycle**: Created during Transaction mode, fulfilled during Confirmation mode

## Data Flow Architecture

```
External Data → Staging Entry → Rule Orchestration → Processing Decision
                                                      ↓
                Transaction Mode ← → Confirmation Mode
                      ↓                           ↓
            Create Transaction          Find Expectations
            Create Expectations        Validate Matches
                      ↓                           ↓
                Search Expectations ← → Transaction Evolution
```

## Processing Modes

### Transaction Mode
- **Trigger**: Staging entry with `processing_mode: Transaction`
- **Flow**: Rule matching → Transaction creation → Search expectation creation
- **Result**: New transaction with expected entries awaiting confirmation

### Confirmation Mode
- **Trigger**: Staging entry with `processing_mode: Confirmation`
- **Flow**: Expectation search → Match validation → Transaction evolution
- **Result**: Existing transaction updated with confirmed entries

## Rule System Architecture

### Rule Orchestration
- **Discovery**: Find rules by account type (source/target) via `recon_rule_account` table
- **Filtering**: Apply trigger conditions to select applicable rules
- **Selection**: Choose highest priority rule among matches
- **Execution**: Route to appropriate strategy implementation

### Strategy Pattern
- **Interface**: `ReconEngine` trait defines standard processing methods
- **Strategy Categories**:
  - **OneToOne**: 1 source account → 1 target account
    - **SingleSingle**: 1 source entry → 1 target entry
    - **SingleMany**: 1 source entry → many target entries
    - **ManySingle**: Many source entries → 1 target entry (with grouping field)
    - **ManyMany**: Many source entries → many target entries (with grouping field)
  - **OneToMany**: 1 source account → multiple target accounts
    - **SingleSingle**: 1 source entry → 1 entry per target account (with split logic)
- **Shared Utilities**: Common functions extracted to `utils::recon_engine` for code reuse
  - **Balance Calculation**: `get_remaining_entry_direction_and_amount` shared across strategies
  - **Trait Generalization**: `ReconEngineSearchGroupInterface` returns boxed enum variants
- **Versioning**: All strategies support versioned evolution
- **Extension**: New strategies can be added without core changes

## Versioning System

### Staging Entry Versioning
- **Primary Key**: `id` (unique per version)
- **Logical ID**: `staging_entry_id` (groups versions)
- **Version Field**: Incrementing integer for each new version
- **Archive Pattern**: Old versions marked with `discarded_at` timestamp

### Transaction Versioning
- **Evolution**: New versions created for status changes
- **Continuity**: Same `transaction_id` across versions
- **Archive**: Old versions preserved with `Archived` status
- **Atomic**: All-or-nothing version transitions

## Security Architecture

### Encryption
- **Metadata**: All metadata encrypted with merchant-specific keys
- **Key Derivation**: Uses `Identifier::ReconMerchant(merchant_id)`
- **Storage**: Encrypted data stored as BYTEA, not JSONB
- **Access**: Decryption only in domain layer for business logic

### Authorization
- **Profile Scoping**: All operations scoped to business profile
- **Account Validation**: Accounts must belong to authenticated profile
- **Rule Access**: Rules inherit profile from associated accounts

## Error Handling Architecture

### Processing Errors
- **Business Logic**: Missing fields, no rules, validation failures
- **System Errors**: Database failures, encryption issues
- **Recovery**: Manual review status for unprocessable entries

### Error Context
- **Structured Errors**: All errors include context and field details
- **Audit Trail**: Error reasons stored in entry data
- **Recovery Paths**: Clear escalation procedures for different error types

## Performance Architecture

### Batch Operations
- **Account Validation**: HashMap-based O(1) lookups
- **Rule Discovery**: Indexed queries via `recon_rule_account`
- **Search Expectations**: Composite indexes for efficient matching

### Concurrency
- **Atomic Transactions**: All operations use database transactions
- **Optimistic Locking**: Version fields prevent concurrent modifications
- **Batch Processor**: Background processing of pending entries

## Integration Points

### External Systems
- **Transformation**: Links to transformation configs and execution history
- **Ingestion**: Tracks file lineage through transformation history
- **Encryption**: Integrates with merchant key management

### Internal Systems
- **Accounts**: Validates account ownership and currency
- **Business Profiles**: Provides multi-tenant isolation
- **Audit Trail**: Records all state changes for compliance
