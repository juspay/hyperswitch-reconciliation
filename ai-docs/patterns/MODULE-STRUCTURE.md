# Module Structure Pattern

## When to Use
- Creating new feature domains (not infrastructure)
- Adding business logic that doesn't belong in core layer
- Building functionality that won't have direct API routes

## Domain Module Pattern

### Structure
```
src/
├── domain.rs                    # Module exports
└── domain/
    └── feature_name.rs          # Self-contained feature implementation
```

### Example: Rule Orchestration Domain (Simplified)
```rust
// src/domain.rs
pub mod recon_engine;

// src/domain/recon_engine/rule_orchestration.rs
use crate::routes::AppState;
use api_models::recon_engine::ProcessingDecision;
use common_enums::StagingEntryProcessingMode;
use diesel_models::staging_entry::StagingEntry;

// Simple synchronous function - no async needed
pub fn rule_orchestration(
    _store: &AppState,
    staging_entry: &StagingEntry,
) -> ProcessingDecision {
    match staging_entry.processing_mode {
        StagingEntryProcessingMode::Transaction => ProcessingDecision::ProcessAsTransaction,
        StagingEntryProcessingMode::Confirmation => ProcessingDecision::ProcessAsConfirmation,
    }
}
```

## Key Principles

### Self-Contained
- All related functionality in one file initially
- Split only when file exceeds ~500 lines
- Clear public interface through domain.rs exports

### No Direct API Routes
- Domain modules accessed through other layers
- Called from background jobs, core layer, or services
- Business logic, not web interface

### Database Interface Usage
- Use existing db interface traits
- Don't bypass Store layer for direct DB access
- Handle database errors appropriately

### Error Handling
- Use domain-appropriate error types (ReconAppError)
- Provide meaningful error context
- Follow error-stack pattern for error propagation

## Library Module Updates

### Update lib.rs
```rust
pub mod domain;  // Add to existing modules
```

### Follow Existing Pattern
- Look at core/ module structure for reference
- Use same naming conventions
- Follow same import/export patterns

## Testing Strategy

### Domain Module Tests
- Test business logic in isolation
- Mock Store interface for unit tests
- Integration tests with real database

### File Organization
```
tests/
└── domain/
    └── rule_orchestration_test.rs
```

## Common Gotchas

### File Structure
- Use domain.rs + domain/ folder, not mod.rs
- Keep exports minimal and focused
- Don't expose internal implementation details

### Dependencies
- Import store interfaces from the `storage_impl` crate and errors from crate::errors
- Use interface traits, not concrete implementations
- Avoid circular dependencies with core layer

### Documentation
- Create domain-specific AGENT-NOTES.md
- Update INDEX.md with new domain
- Document cross-domain relationships

## When to Split Files

### Single File Limits
- Keep under ~500 lines for maintainability
- Split by logical functionality, not arbitrary line count
- Maintain cohesive public interface

### Split Strategy
```
domain/
├── recon_engine.rs            # Module exports
└── recon_engine/
    ├── rule_orchestration.rs  # Simple processing mode decision
    └── processors.rs          # Future: actual processing logic
```

## External Services Pattern

<!-- AI-UPDATED: 2025-01-07 - Documented secret management consolidation -->

### Consolidated Structure
External service integrations are consolidated in the `external_services` crate:

```
external_services/
├── src/
│   ├── lib.rs                           # Re-exports main interfaces
│   └── secrets/                         # Secret management module
│       ├── mod.rs                       # Module exports
│       ├── interfaces.rs                # Core traits and errors
│       ├── state.rs                     # Secret state management
│       ├── handlers.rs                  # Secret handler traits
│       ├── config.rs                    # Configuration management
│       └── implementations/             # Concrete implementations
│           ├── mod.rs
│           ├── no_encryption.rs         # Development/testing
│           ├── aws_kms.rs              # AWS KMS integration
│           └── hashicorp_vault.rs      # HashiCorp Vault integration
```

### Usage Pattern
```rust
// In consumer crates (e.g., recon)
use external_services::secrets::{
    SecretManagementInterface,
    SecretsManagementConfig,
    SecretStateContainer,
    RawSecret,
    SecuredSecret,
};
```

### Key Benefits
- **Single dependency**: One crate instead of separate interface/implementation crates
- **Feature flags**: Optional backends (aws_kms, hashicorp_vault, no_encryption)
- **Extensible**: Room for future external service integrations
- **Clean imports**: All related functionality in one namespace

### Gotchas
- **Import paths**: Use `external_services::secrets::*` not root-level imports
- **Feature selection**: Must specify features in Cargo.toml (e.g., `features = ["no_encryption"]`)
- **Configuration**: All secret backends configured through `SecretsManagementConfig` enum

## Anti-Patterns

### Don't Create
- ❌ Direct API routes in domain modules
- ❌ Database models in domain layer
- ❌ Infrastructure concerns (logging setup, config)
- ❌ Separate interface/implementation crates for external services

### Don't Bypass
- ❌ Store interface for direct DB access
- ❌ Error handling with unwrap() or panic!()
- ❌ Type system with unsafe code
- ❌ External services crate structure - use the consolidated approach

## Integration Points

### API Integration
```rust
// In core/recon_engine.rs
use crate::domain::recon_engine::rule_orchestration;

pub async fn trigger_recon_engine(
    state: &AppState,
    auth: AuthContext,
    request: ReconEngineTriggerRequest,
) -> ReconEngineResult<ReconEngineTriggerResponse> {
    // Load staging entry from DB
    let staging_entry = state.store
        .find_staging_entry_by_id(&request.staging_entry_id)
        .await?;
    
    // Call synchronous rule orchestration
    let processing_decision = rule_orchestration(state, &staging_entry);
    
    // Return decision in API response
    Ok(ReconEngineTriggerResponse {
        staging_entry_id: request.staging_entry_id,
        processing_decision,
    })
}
```

### Background Jobs
```rust
use crate::domain::recon_engine::rule_orchestration;
use api_models::recon_engine::ProcessingDecision;

async fn process_staging_entries(state: &AppState) {
    let entries = load_pending_entries().await?;
    
    for entry in entries {
        let decision = rule_orchestration(state, &entry);
        
        match decision {
            ProcessingDecision::ProcessAsTransaction => {
                // Handle transaction processing
            }
            ProcessingDecision::ProcessAsConfirmation => {
                // Handle confirmation processing
            }
        }
    }
}
```
