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
    .attach_printable("Failed during account validation")?;
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

// 2. Implement error code mapping
impl AccountError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::AccountNotFound => "ACCOUNT_NOT_FOUND",
            Self::AccountNameEmpty => "INVALID_ACCOUNT_NAME",
            Self::InsufficientBalance => "INSUFFICIENT_BALANCE",
            Self::CurrencyMismatch { .. } => "CURRENCY_MISMATCH",
            Self::InternalServerError => "INTERNAL_ERROR",
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
        .attach_printable_lazy(|| {
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
        .attach_printable_lazy(|| format!("Source account not found: {from_id}"))?;
    
    // Get destination account
    let to_account = state
        .store
        .find_account_by_id(to_id)
        .await
        .to_not_found_response(|| AccountError::AccountNotFound)
        .attach_printable_lazy(|| format!("Destination account not found: {to_id}"))?;
    
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
        .attach_printable("Database transaction failed during transfer")?;
    
    Ok(TransferResponse::from(transfer))
}

// 5. Error response formatting for API
#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error_code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl From<&AccountError> for ErrorResponse {
    fn from(error: &AccountError) -> Self {
        Self {
            error_code: error.error_code().to_string(),
            message: error.to_string(),
            details: match error {
                AccountError::CurrencyMismatch { expected, actual } => {
                    Some(serde_json::json!({
                        "expected_currency": expected,
                        "actual_currency": actual,
                    }))
                }
                _ => None,
            },
        }
    }
}

// 6. HTTP status code mapping
impl AccountError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::AccountNotFound => StatusCode::NOT_FOUND,
            Self::AccountNameEmpty => StatusCode::BAD_REQUEST,
            Self::InsufficientBalance => StatusCode::BAD_REQUEST,
            Self::CurrencyMismatch { .. } => StatusCode::BAD_REQUEST,
            Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// 7. Using in route handler with server_wrap
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

## Key Points
- **Use error-stack** - Provides rich error context and chaining
- **Module-specific errors** - Each module has its own error enum
- **StorageErrorExt trait** - Converts DB errors appropriately
- **change_context** - Transform errors between types
- **attach_printable** - Add debugging context
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
- attach_printable adds context without changing error type
- server_wrap converts any error to HTTP response