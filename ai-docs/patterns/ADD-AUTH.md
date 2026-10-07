# Pattern: Add Authentication

## When to Use
- Protecting API endpoints that require authentication
- Validating JWT tokens in requests
- Ensuring merchant/profile ID matches the authenticated user
- Implementing different levels of access control
- Extracting authentication context for business logic
<!-- AI-UPDATED: 2025-01-07 - Admin API key authentication -->
- Protecting administrative endpoints with API key authentication
- Bypassing JWT validation for system-level operations

## Quick Example

### JWT Authentication
```rust
use crate::services::auth::{JwtAuth, AuthValidator};

pub async fn protected_endpoint(
    state: web::Data<AppState>,
    path: web::Path<String>, // merchant_id from path
    req: HttpRequest,
) -> impl Responder {
    let merchant_id = path.into_inner();
    let auth = JwtAuth::with_merchant(merchant_id.clone());
    
    // Validate and get context
    let auth_context = auth
        .validate(&req, state.as_ref().clone())
        .await
        .map_err(|_| HttpResponse::Unauthorized().finish())?;
    
    // Use auth_context.merchant_id and auth_context.profile_id
    // ... handler logic
}
```

### Admin API Key Authentication
<!-- AI-UPDATED: 2025-01-07 - Admin API key pattern -->
```rust
use crate::services::auth::{AdminApiAuth, AuthValidator};

pub async fn admin_endpoint(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<AdminRequest>,
) -> impl Responder {
    let auth = AdminApiAuth;
    
    // Validate admin API key from api-key header
    auth.validate(&req, state.as_ref().clone())
        .await
        .map_err(|_| HttpResponse::Unauthorized().finish())?;
    
    // No auth context returned - admin operations don't need merchant/profile scope
    // ... admin logic
}
```

## Full Implementation
```rust
// Import authentication components
use crate::services::auth::{JwtAuth, AuthValidator};
use common_utils::auth::AuthContext;
use actix_web::{web, HttpRequest, HttpResponse, Responder};

// 1. Basic authentication - any valid JWT
pub async fn list_all_resources(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Any authenticated user can access
    let auth = JwtAuth::basic();
    
    let auth_context = auth
        .validate(&req, state.as_ref().clone())
        .await
        .map_err(|_| HttpResponse::Unauthorized().finish())?;
    
    // auth_context contains merchant_id and profile_id from JWT
    core::resource::list_for_profile(
        state.as_ref(),
        &auth_context.profile_id,
    )
    .await
    .server_wrap("list_resources")
    .await
}

// 2. Merchant-level authentication
pub async fn get_merchant_details(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let merchant_id = path.into_inner();
    
    // JWT must be for this specific merchant
    let auth = JwtAuth::with_merchant(merchant_id.clone());
    
    let auth_context = auth
        .validate(&req, state.as_ref().clone())
        .await
        .map_err(|_| HttpResponse::Unauthorized().finish())?;
    
    core::merchant::get_merchant(
        state.as_ref(),
        &merchant_id,
    )
    .await
    .server_wrap("get_merchant")
    .await
}

// 3. Profile-level authentication
pub async fn get_profile_accounts(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let profile_id = path.into_inner();
    
    // JWT must be for this specific profile
    let auth = JwtAuth::with_profile(profile_id.clone());
    
    let auth_context = auth
        .validate(&req, state.as_ref().clone())
        .await
        .map_err(|_| HttpResponse::Unauthorized().finish())?;
    
    core::account::list_accounts_for_profile(
        state.as_ref(),
        &profile_id,
    )
    .await
    .server_wrap("list_accounts")
    .await
}

// 4. Both merchant and profile validation
pub async fn create_transaction(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>, // (merchant_id, profile_id)
    req: HttpRequest,
    body: web::Json<TransactionRequest>,
) -> impl Responder {
    let (merchant_id, profile_id) = path.into_inner();
    
    // JWT must match both merchant AND profile
    let auth = JwtAuth::with_merchant_and_profile(
        merchant_id.clone(),
        profile_id.clone(),
    );
    
    let auth_context = auth
        .validate(&req, state.as_ref().clone())
        .await
        .map_err(|_| HttpResponse::Unauthorized().finish())?;
    
    core::transaction::create(
        state.as_ref(),
        body.into_inner(),
        &auth_context,
    )
    .await
    .server_wrap("create_transaction")
    .await
}

// 5. OpenAPI documentation with authentication
#[utoipa::path(
    get,
    path = "/merchant_accounts/{merchant_id}/profiles",
    tag = "Business Profiles",
    params(
        ("merchant_id" = String, Path, description = "Merchant ID")
    ),
    security(("Bearer" = [])),
    responses(
        (status = 200, description = "List of profiles", body = Vec<ProfileResponse>),
        (status = 401, description = "Unauthorized - invalid or missing JWT token"),
        (status = 403, description = "Forbidden - JWT doesn't match merchant_id"),
    ),
)]
pub async fn list_merchant_profiles(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let merchant_id = path.into_inner();
    let auth = JwtAuth::with_merchant(merchant_id.clone());
    
    let auth_context = auth
        .validate(&req, state.as_ref().clone())
        .await
        .map_err(|_| HttpResponse::Unauthorized().finish())?;
    
    // Business logic using auth_context
    core::profile::list_for_merchant(
        state.as_ref(),
        &merchant_id,
    )
    .await
    .server_wrap("list_profiles")
    .await
}

// 6. Error handling for auth failures
pub async fn protected_with_error_details(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let auth = JwtAuth::basic();
    
    match auth.validate(&req, state.as_ref().clone()).await {
        Ok(auth_context) => {
            // Proceed with authenticated request
            core::some_operation(state.as_ref(), &auth_context)
                .await
                .server_wrap("operation")
                .await
        }
        Err(e) => {
            // Log the specific error
            tracing::warn!("Authentication failed: {:?}", e);
            
            // Return appropriate error response
            match e.current_context() {
                AuthError::MissingToken => {
                    HttpResponse::Unauthorized()
                        .json(ErrorResponse {
                            error: "missing_token".to_string(),
                            message: "Authorization header required".to_string(),
                        })
                }
                AuthError::InvalidToken => {
                    HttpResponse::Unauthorized()
                        .json(ErrorResponse {
                            error: "invalid_token".to_string(),
                            message: "JWT token is invalid or expired".to_string(),
                        })
                }
                AuthError::MerchantMismatch => {
                    HttpResponse::Forbidden()
                        .json(ErrorResponse {
                            error: "merchant_mismatch".to_string(),
                            message: "Access denied for this merchant".to_string(),
                        })
                }
                _ => HttpResponse::InternalServerError().finish(),
            }
        }
    }
}
```

## Key Points

### JWT Authentication
- **Four auth levels**: basic, with_merchant, with_profile, with_merchant_and_profile
- **Authorization header**: Token extracted from `Authorization: Bearer <token>`
- **Path validation**: Ensures JWT claims match path parameters
- **AuthContext returned**: Contains merchant_id and profile_id from validated JWT
- **Early return on failure**: Use `?` or explicit error handling
- **OpenAPI security**: Add `security(("Bearer" = []))` to protected endpoints

### Admin API Key Authentication
<!-- AI-UPDATED: 2025-01-07 - Admin API key documentation -->
- **Header-based**: Uses `api-key` header, NOT `Authorization: Bearer`
- **No JWT validation**: Bypasses JWT token validation entirely
- **No auth context**: Returns `()` - admin operations don't need merchant/profile scope
- **Simple comparison**: Validates against configured admin_api_key
- **OpenAPI security**: Add `security(("api_key" = []))` to admin endpoints
- **Use cases**: Merchant creation, bootstrap, token generation, system operations

### Internal API Key Authentication
- **Header-based**: `x-internal-api-key`, compared with `auth.internal_api_key` 
- **No auth context**: `InternalApiKeyAuth` returns `()`; use it only for service-to-service calls such as the ingestion → transformation loopback
- **Outside `server_wrap`**: there is no shared helper; compare the header with `get_header_value_by_key` inline (see `authenticate_sftp_internal`) and reject an empty configured key
- **OpenAPI security**: `security(("InternalApiKey" = []))`

## Common Mistakes

### JWT Authentication
- Not validating path parameters against JWT claims
- Using wrong auth level (e.g., basic when merchant validation needed)
- Forgetting to add security to OpenAPI documentation
- Not handling auth errors gracefully
- Passing raw JWT instead of using AuthContext
- Validating auth after business logic instead of before

### Internal API Key Authentication
- Reusing the admin key for service-to-service calls; a leaked admin key could then create merchants
- Leaving a system-triggered route on `NoAuth` when the caller can send the key

### Admin API Key Authentication
<!-- AI-UPDATED: 2025-01-07 - Admin auth gotchas -->
- Using `Authorization: Bearer` header instead of `api-key` header
- Trying to extract AuthContext from AdminApiAuth (returns `()`)
- Using AdminApiAuth for user-scoped operations (use JWT instead)
- Forgetting to configure admin_api_key in environment configs
- Not using `Secret<T>` wrapper when accessing config values

## Related Patterns
- [ADD-ENDPOINT](ADD-ENDPOINT.md) - For adding new protected endpoints
- [ERROR-HANDLING](ERROR-HANDLING.md) - For auth error responses
- [DATABASE-QUERY](DATABASE-QUERY.md) - Using auth context in queries

## Gotchas to Remember

### JWT Authentication
- JWT tokens expire - handle expiration gracefully
- Profile ID is optional in JWT - check if needed
- Auth validation is async - don't forget await
- Different endpoints need different auth levels
- Always validate before any business logic
- Log auth failures for security monitoring

### Admin API Key Authentication
<!-- AI-UPDATED: 2025-01-07 - Admin auth gotchas -->
- Admin API keys don't expire (unlike JWT tokens)
- Admin endpoints bypass all JWT validation
- Use `.peek()` to access secret values in config
- Admin operations are system-wide, not merchant-scoped
- No rate limiting built-in - consider adding if needed
- Relies on HTTPS for transport security
