# Pattern: Add New API Endpoint

## When to Use
- Creating any new REST endpoint
- Adding new functionality to the API
- Exposing new business operations

## Quick Example
```rust
// Route handler - thin layer
// Note: Route paths use snake_case (e.g., /merchant_accounts, /recon_engine)
pub async fn create_thing(
    state: web::Data<AppState>,
    req: Json<CreateThingRequest>,
) -> impl Responder {
    core::thing::create_thing(state.as_ref(), req.into_inner())
        .await
        .server_wrap()
        .await
}
```

## Full Implementation

### 1. Define API Models
```rust
// In crates/api_models/src/thing.rs
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateThingRequest {
    #[schema(example = "My Thing")]
    pub name: String,
    
    #[schema(example = "Description of thing")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ThingResponse {
    pub thing_id: String,
    pub name: String,
    pub description: Option<String>,
}

// Export in crates/api_models/src/lib.rs
pub mod thing;
```

### 2. Create Route Handler
```rust
// In crates/recon/src/routes/thing.rs
use actix_web::{web, HttpRequest};
use api_models::thing::{CreateThingRequest, ThingResponse};

use crate::{
    core,
    routes::{app::AppState, ServerWrap},
};

#[utoipa::path(
    post,
    path = "/things",  // Use snake_case: /user_settings, /payment_methods
    tag = "Thing Management",
    request_body = CreateThingRequest,
    responses(
        (status = 201, description = "Thing created successfully", body = ThingResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_thing(
    state: web::Data<AppState>,
    req: web::Json<CreateThingRequest>,
    http_req: HttpRequest, // if auth needed
) -> impl actix_web::Responder {
    // For authenticated endpoints:
    let auth = JwtAuth::with_merchant(merchant_id);
    let auth_context = auth
        .validate(&http_req, state.as_ref().clone())
        .await
        .map_err(|_| HttpResponse::Unauthorized().finish())?;
    
    core::thing::create_thing(
        state.as_ref(), 
        req.into_inner(),
        &auth_context.profile_id, // if needed
    )
    .await
    .server_wrap()
    .await
}

// Export in crates/recon/src/routes/mod.rs
pub mod thing;
```

### 3. Implement Core Business Logic
```rust
// In crates/recon/src/core/thing.rs
use api_models::thing::{CreateThingRequest, ThingResponse};
use diesel_models::thing::ThingNew;
use error_stack::ResultExt;

use crate::{
    db::thing::ThingInterface,
    errors::thing::{ThingError, ThingResult},
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn create_thing(
    state: &AppState,
    request: CreateThingRequest,
    profile_id: &str, // if scoped to profile
) -> ThingResult<ThingResponse> {
    // Validation
    if request.name.trim().is_empty() {
        return Err(ThingError::NameEmpty.into());
    }
    
    // Generate ID
    let thing_id = {
        let id = nanoid::nanoid!(10);
        format!("thing_{id}")
    };
    
    // Create database model
    let thing_new = ThingNew {
        thing_id: thing_id.clone(),
        name: request.name.clone(),
        description: request.description,
        profile_id: profile_id.to_string(),
        created_at: common_utils::date_time::now(),
        last_modified_at: common_utils::date_time::now(),
    };
    
    // Insert via store
    let thing = state
        .store
        .insert_thing(thing_new)
        .await
        .change_context(ThingError::InternalServerError)?;
    
    // Return response
    Ok(ApplicationResponse::json_created(ThingResponse {
        thing_id: thing.thing_id,
        name: thing.name,
        description: thing.description,
    }))
}

// Export in crates/recon/src/core/mod.rs
pub mod thing;
```

### 4. Define Errors
```rust
// In crates/recon/src/errors/thing.rs
use error_stack::ResultExt;

use crate::routes::errors::types::{api::ErrorCode, ApplicationResponse};

#[derive(Debug, thiserror::Error)]
pub enum ThingError {
    #[error("Thing name cannot be empty")]
    NameEmpty,
    
    #[error("Thing not found")]
    NotFound { thing_id: String },
    
    #[error("An internal error occurred")]
    InternalServerError,
}

impl ErrorCode for ThingError {
    fn code(&self) -> &'static str {
        match self {
            Self::NameEmpty => "THING_001",
            Self::NotFound { .. } => "THING_002", 
            Self::InternalServerError => "THING_500",
        }
    }
    
    fn message(&self) -> &'static str {
        match self {
            Self::NameEmpty => "Thing name is required",
            Self::NotFound { .. } => "Thing not found",
            Self::InternalServerError => "An error occurred",
        }
    }
}

pub type ThingResult<T> = error_stack::Result<ApplicationResponse<T>, ThingError>;
```

### 5. Register the Endpoint
```rust
// In crates/recon/src/routes/app.rs
// IMPORTANT: Use snake_case for all routes (not kebab-case)
pub fn app_router(state: AppState) -> Scope {
    web::scope("/api/v1")
        // ... existing routes
        .service(
            web::resource("/things")  // ✓ Correct: snake_case
                .route(web::post().to(thing::create_thing))
        )
        // NOT: web::resource("/thing-items")  // ✗ Wrong: kebab-case
}

// In crates/recon/src/openapi.rs
#[derive(OpenApi)]
#[openapi(
    paths(
        // ... existing paths
        crate::routes::thing::create_thing,
    ),
    components(schemas(
        // ... existing schemas
        api_models::thing::CreateThingRequest,
        api_models::thing::ThingResponse,
    ))
)]
pub struct ApiDoc;
```

## Key Points

1. **Layer Separation**: Routes are thin, Core has business logic, DB only does data access
2. **Error Handling**: Use error-stack with proper context
3. **Response Types**: Always use ApplicationResponse, never raw DB models
4. **ID Generation**: Server-side with format "type_nanoid(10)"
5. **Validation**: In Core layer, not route or DB
6. **Authentication**: Add JwtAuth when endpoint needs auth
7. **Route Naming**: Always use snake_case for routes (e.g., `/staging_entries`, `/recon_engine`)

## Common Variations

### GET Endpoint
```rust
// Route
pub async fn get_thing(
    state: web::Data<AppState>,
    path: web::Path<String>, // thing_id
) -> impl Responder {
    let thing_id = path.into_inner();
    core::thing::get_thing(state.as_ref(), &thing_id)
        .await
        .server_wrap()
        .await
}

// Core - note the error handling
let thing = state
    .store
    .find_thing_by_id(&thing_id)
    .await
    .to_not_found_response(|| ThingError::NotFound { 
        thing_id: thing_id.clone() 
    })?;
```

### LIST Endpoint with Filters
```rust
// Add query params
pub async fn list_things(
    state: web::Data<AppState>,
    query: web::Query<ListThingsQuery>,
) -> impl Responder {
    core::thing::list_things(
        state.as_ref(),
        query.into_inner(),
    )
    .await
    .server_wrap()
    .await
}
```

## Profile-Scoped Endpoints

When adding endpoints that operate on profile-specific data:

```rust
// Use JwtAuth::with_profile for path-based profile validation
pub async fn list_things_by_profile_path(
    state: web::Data<AppState>,
    path: web::Path<String>, // profile_id
    http_req: HttpRequest,
) -> impl Responder {
    let profile_id = path.into_inner();
    api::server_wrap(
        JwtAuth::with_profile(profile_id.clone()),
        &http_req,
        state,
        |state, _auth| async move {
            core::thing::list_things_by_profile_id(&state, &profile_id).await
        },
    )
    .await
}
```

**Key Points:**
<!-- AI-UPDATED: 2024-06-24 - Added profile-scoped endpoint pattern -->
- Use `JwtAuth::with_profile(profile_id)` to ensure user can only access their own profile's data
- Returns 401 if JWT profile_id doesn't match the path profile_id
- Register under business_profiles scope: `/business_profiles/{profile_id}/things`
- Provides alternative access pattern to JWT-scoped endpoints

## Gotchas to Remember
- Never put business logic in route handlers (see ai-docs/RULES.md)
- Always validate in Core, not DB layer
- Use proper error conversion with to_not_found_response()
- Remember to export new modules and register endpoints
- For profile-scoped paths, always use `JwtAuth::with_profile()` for security
- NoAuth endpoints are publicly accessible - use only for system-level operations
