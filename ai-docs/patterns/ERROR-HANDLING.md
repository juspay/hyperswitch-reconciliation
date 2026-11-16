# Pattern: Error Handling

## When to Use
- Converting database errors to appropriate HTTP responses
- Adding context to errors for better debugging
- Creating custom error types for new modules
- Handling validation errors gracefully
- Implementing error codes for client consumption

## Quick Example
```rust
use error_stack::{Report, ResultExt};

// Convert database error to 404
let account = state
    .store
    .find_account_by_id(&account_id)
    .await
    .to_not_found_response(|| AccountError::AccountNotFound)?;

// Add context to any error
let result = some_operation()
    .change_context(AccountError::InternalServerError)
    .attach("Failed during account validation")?;
```

## Full Implementation
```rust
// 1. Define module-specific error enum
#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("Account not found")]
    AccountNotFound,
    
    #[error("Account name cannot be empty")]
    AccountNameEmpty,
    
    #[error("Insufficient balance")]
    InsufficientBalance,
    
    #[error("Currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch {
        expected: Currency,
        actual: Currency,
    },
    
    #[error("An internal error occurred")]
    InternalServerError,
}

// 2. Map each variant to an HTTP status (the payload variant) and its code
use api_models::errors::{ApiError, ApiErrorResponse as ApiErrorPayload};
use common_utils::errors::ErrorSwitchFrom;

impl ErrorSwitchFrom<AccountError> for ApiErrorPayload {
    fn switch_from(error: &AccountError) -> Self {
        match error {
            AccountError::InternalServerError => Self::InternalServerError(ApiError::new(
                "RE",
                300,
                "An internal error occurred while processing account request",
                None,
            )),
            AccountError::AccountNotFound => {
                Self::NotFound(ApiError::new("RE", 302, "Account not found", None))
            }
            AccountError::AccountNameEmpty => {
                Self::BadRequest(ApiError::new("RE", 304, "Account name cannot be empty", None))
            }
            // ...one arm per variant, each with its own code in block 3 (see Error Codes)
        }
    }
}

// 3. Use StorageErrorExt for database operations
use crate::errors::StorageErrorExt;

pub async fn find_account(
    state: &AppState,
    account_id: &str,
) -> AccountResult<Account> {
    // Automatically converts NotFound to AccountNotFound
    let account = state
        .store
        .find_account_by_id(account_id)
        .await
        .to_not_found_response(|| AccountError::AccountNotFound)?;
    
    Ok(account)
}

pub async fn create_account(
    state: &AppState,
    request: AccountCreateRequest,
) -> AccountResult<AccountResponse> {
    // Validation with specific error
    if request.account_name.is_empty() {
        return Err(AccountError::AccountNameEmpty.into());
    }
    
    let account_new = AccountNew {
        account_id: {
            let id = nanoid::nanoid!(10);
            format!("account_{id}")
        },
        account_name: request.account_name,
        // ... other fields
    };
    
    // Handle duplicate account gracefully
    let account = state
        .store
        .insert_account(account_new)
        .await
        .to_duplicate_response(|| AccountError::AccountAlreadyExists)
        .attach_with(|| {
            format!("Attempted to create duplicate account: {account_id}")
        })?;
    
    Ok(AccountResponse::from(account))
}

// 4. Complex operation with multiple error contexts
pub async fn transfer_between_accounts(
    state: &AppState,
    from_id: &str,
    to_id: &str,
    amount: AmountType,
) -> AccountResult<TransferResponse> {
    // Get source account
    let from_account = state
        .store
        .find_account_by_id(from_id)
        .await
        .to_not_found_response(|| AccountError::AccountNotFound)
        .attach_with(|| format!("Source account not found: {from_id}"))?;
    
    // Get destination account
    let to_account = state
        .store
        .find_account_by_id(to_id)
        .await
        .to_not_found_response(|| AccountError::AccountNotFound)
        .attach_with(|| format!("Destination account not found: {to_id}"))?;
    
    // Validate currencies match
    if from_account.currency != amount.currency() {
        return Err(AccountError::CurrencyMismatch {
            expected: from_account.currency,
            actual: amount.currency(),
        }.into());
    }
    
    // Check sufficient balance
    if from_account.posted_balance < amount.value() {
        return Err(AccountError::InsufficientBalance.into());
    }
    
    // Perform transfer in transaction
    let transfer = state
        .store
        .transfer_funds(from_id, to_id, amount)
        .await
        .change_context(AccountError::InternalServerError)
        .attach("Database transaction failed during transfer")?;
    
    Ok(TransferResponse::from(transfer))
}

// 5. Using in route handler with server_wrap
pub async fn handle_transfer(
    state: web::Data<AppState>,
    body: web::Json<TransferRequest>,
) -> impl Responder {
    core::account::transfer_between_accounts(
        state.as_ref(),
        &body.from_account_id,
        &body.to_account_id,
        body.amount.clone(),
    )
    .await
    .server_wrap("transfer_funds")
    .await
}
```

## Error Codes
- `ApiError::new("RE", 302, …)` renders as `RE_302`: `{prefix}_{number}`, zero-padded to two digits (`RE_01`). The number is `block × 100 + local`. All error types live in `crates/recon/src/errors/`.
- **Each error type owns one block, unique across prefixes.** The prefix doesn't namespace the number, so the number alone identifies the error type. A new error type takes the next free block, 22.
- **Local numbers only grow; never renumber or reuse one.** Anything matching on a code would silently get a different error. An unused number inside a block is a retired code.

| Block | Error type | Block | Error type |
|---|---|---|---|
| 0 | `ReconAppError` | 11 | `MetadataSchemaError` |
| 1 | `MerchantAccountError` | 12 | `AuditTrailError` |
| 2 | `BusinessProfileError` | 13 | `SearchGroupError` |
| 3 | `AccountError` | 14 | `BulkOperationError` |
| 4 | `StagingEntryError` | 15 | `SearchExpectationError` |
| 5 | `EntryError` | 16 | `OverviewError` |
| 6 | `TransactionError` | 17 | `SearchEntryError` |
| 7 | `ReconRuleError` (`RR`) | 18 | `ReportError` (`RP`) |
| 8 | `ReconEngineError` | 19 | `LinkConfigError` (`LC`) |
| 9 | `IngestionError` (`IE`) | 20 | `LinkingProcessorError` (`LP`) |
| 10 | `TransformationError` (`TR`) | 21 | `ApiKeyError` |

The prefix is `RE` unless shown.

## Key Points
- **Use error-stack** - Provides rich error context and chaining
- **Module-specific errors** - Each module has its own error enum
- **StorageErrorExt trait** - Converts DB errors appropriately
- **change_context** - Transform errors between types
- **attach** - Add debugging context
- **Error codes** - Machine-readable codes for clients
- **server_wrap** - Handles error to HTTP response conversion

## Common Mistakes
- Using generic errors instead of specific ones
- Not converting DatabaseError::NotFound to entity-specific NotFound
- Losing error context during conversions
- Returning 500 for client errors (should be 4xx)
- Not providing error codes for client handling
- Using unwrap() instead of proper error handling

## Related Patterns
- [ADD-ENDPOINT](ADD-ENDPOINT.md) - Error handling in routes
- [DATABASE-QUERY](DATABASE-QUERY.md) - Database error conversion
- [ADD-AUTH](ADD-AUTH.md) - Authentication error handling

## Gotchas to Remember
- Always use `?` operator - it preserves error stack traces
- to_not_found_response for GET operations
- to_duplicate_response for CREATE operations
- change_context loses original error type (but keeps message)
- attach adds context without changing error type
- server_wrap converts any error to HTTP response