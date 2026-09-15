# Import Style Patterns

## Rule: Use Short, Clean Imports

### ❌ Bad - Fully Qualified Paths in Code
```rust
// Don't use fully qualified paths in match statements
match result {
    diesel_models::recon_engine::MatchResult::Matched { .. } => { ... }
    diesel_models::recon_engine::MatchResult::Mismatched { details } => { ... }
}
```

### ✅ Good - Import and Use Short Names
```rust
// Import the types at the top
use diesel_models::recon_engine::{MatchResult, ProcessingResult};

// Use short names in code
match result {
    MatchResult::Matched { .. } => { ... }
    MatchResult::Mismatched { details } => { ... }
}
```

## Import Organization

### 1. Standard Library Imports
```rust
use std::collections::HashMap;
use std::sync::Arc;
```

### 2. External Crate Imports
```rust
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use error_stack::{Report, ResultExt};
```

### 3. Internal Crate Imports (from other crates in workspace)
```rust
use diesel_models::recon_engine::{MatchResult, ProcessingResult};
use common_enums::{EntryStatus, TransactionStatus};
use api_models::recon_engine::ProcessingDecision;
```

### 4. Local Module Imports (from current crate)
```rust
use crate::{
    db::{account::AccountInterface, transaction::TransactionInterface},
    errors::recon_engine::ReconEngineError,
    routes::AppState,
};
```

## Grouping Rules

1. **Group Related Imports**: Import multiple items from the same module in one line
   ```rust
   // ✅ Good
   use diesel_models::recon_engine::{MatchResult, ProcessingResult, ReconciliationResult};
   
   // ❌ Bad
   use diesel_models::recon_engine::MatchResult;
   use diesel_models::recon_engine::ProcessingResult;
   use diesel_models::recon_engine::ReconciliationResult;
   ```

2. **Use Nested Imports for Local Modules**:
   ```rust
   use crate::{
       db::{account::AccountInterface, search_expectation::SearchExpectationInterface},
       errors::recon_engine::ReconEngineError,
       utils::recon_engine::{prepare_mismatch_result, validate_match_staging},
   };
   ```

3. **Separate Groups with Blank Lines**:
   ```rust
   use std::collections::HashMap;
   
   use serde::{Deserialize, Serialize};
   use diesel::prelude::*;
   
   use diesel_models::recon_engine::MatchResult;
   use common_enums::EntryStatus;
   
   use crate::{
       db::AccountInterface,
       errors::ReconEngineError,
   };
   ```

## Enum Variant Usage

### ✅ Always Import the Enum and Use Short Names
```rust
use diesel_models::recon_engine::{MatchResult, ProcessingResult};

// In code
match processing_result {
    ProcessingResult::Reconciled(result) => { ... }
    ProcessingResult::Mismatched(result) => { ... }
    ProcessingResult::Unmatched => { ... }
}

match match_result {
    MatchResult::Matched { details } => { ... }
    MatchResult::Mismatched { details } => { ... }
}
```

### ❌ Never Use Fully Qualified Paths in Pattern Matching
```rust
// Don't do this
match processing_result {
    diesel_models::recon_engine::ProcessingResult::Reconciled(result) => { ... }
    diesel_models::recon_engine::ProcessingResult::Mismatched(result) => { ... }
}
```

## When to Use `as` Aliases

Use aliases when there are naming conflicts:
```rust
use diesel_models::entry::Entry as DbEntry;
use api_models::entry::Entry as ApiEntry;
```

## Summary

- **Import at the top, use short names in code**
- **Group related imports from the same module**
- **Organize imports: std → external → internal → local**
- **Use nested imports for local modules**
- **Separate import groups with blank lines**
- **Never use fully qualified paths in pattern matching**
