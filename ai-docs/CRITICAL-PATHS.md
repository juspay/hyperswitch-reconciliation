# Critical User Paths - Read This First

## Authentication & Authorization Flow
<!-- AI-UPDATED: 2026-03-12 - user_id required in JWT claims, token generation removed -->
- **Gotcha**: JWT tokens encode merchant_id, profile_id, AND user_id
- **Gotcha**: Path parameters MUST match JWT claims or request fails
- **Gotcha**: Use JwtAuth::with_merchant() for merchant-scoped endpoints
- **Gotcha**: Bootstrap endpoint creates merchant + profile only (no token returned)
- **Critical**: JWT claims MUST include non-empty `user_id` - validation fails otherwise
- **Critical**: AdminApiAuth uses `api-key` header, NOT `Authorization: Bearer`
- **Critical**: Admin endpoints bypass JWT validation entirely - different auth flow
- **Critical**: Service-to-service routes (transformation trigger) use `InternalApiKeyAuth` with the `x-internal-api-key` header and `auth.internal_api_key`, a different key from `admin_api_key`. Never use `NoAuth` for them
- **Critical**: Admin API key required for merchant creation and bootstrap
- **Critical**: Token generation API removed - clients must generate JWTs themselves using shared secret
- **V2 Pattern**: Transformation Config V2 uses JWT authentication (not AdminApiAuth like V1)
- **V2 Pattern**: V2 endpoints validate ingestion_id ownership via profile_id from JWT context
- **V2 Pattern**: V2 endpoints use AuthContext containing merchant_id, profile_id, and user_id from JWT

## Merchant → Profile → Account Hierarchy
- **Gotcha**: This hierarchy is strictly enforced by foreign keys
- **Gotcha**: Cannot create orphaned entities (profile without merchant, etc.)
- **Gotcha**: Soft deletion doesn't cascade - must handle each level
- **Gotcha**: Profile names need NOT be unique within a merchant (no DB constraint)

## Money Handling
- **Critical**: NEVER use f64 for money - always MinorUnit (i64) in DB
- **Critical**: AmountType handles major ↔ minor unit conversion
- **Critical**: Currency is mandatory for all monetary operations
- **Critical**: Account balances are calculated from debit/credit components: initial_balance + (credits - debits)
- **Critical**: Effective timestamps are mandatory for entries and staging_entries (TIMESTAMP NOT NULL)


## Database Patterns
- **Critical**: All tables use soft delete (discarded_at timestamp)
- **Critical**: Timestamps stay naive UTC; profile timezone is metadata in `business_profile.timezone`, validated in Rust only
- **Critical**: When soft deleting, capture status in discarded_status
- **Critical**: Never use hard deletes - data is immutable for audit
- **Critical**: All IDs are VARCHAR(64) with pattern "{type}_{base62_20}"
  - Format: Base62 (A-Z, a-z, 0-9), 20 characters, no special characters
  - Generated via common_utils::id_generation::generate_id_with_default_len()
  - Example: account_a3D9k2MP8xQ4r7V9

## API Response Patterns
- **Standard**: 201 Created for POST with ApplicationResponse::json_created
- **Standard**: 200 OK for GET/PUT with ApplicationResponse::json
- **Standard**: Always return DTOs, never raw DB models
- **Standard**: Error context added with .change_context()

## Common Integration Points
- **Pattern**: Routes only pass requests to Core layer
- **Pattern**: Core contains all business logic
- **Pattern**: DB layer only does data access
- **Pattern**: Store interface abstracts all DB operations
<!-- AI-UPDATED: 2026-10-22 - Storage layer extracted into storage_impl crate -->
- **Pattern**: `Store` and all `*Interface` traits live in the `storage_impl` crate (mirrors Hyperswitch), not `recon::db` — import them as `storage_impl::account::AccountInterface`
- **Critical**: `storage_impl` must never depend on `recon`; DB logic it needs is written inline in the `impl … for Store` method that uses it

## External Services Integration
<!-- AI-UPDATED: 2025-03-13 - User client architecture refactor -->
- **Pattern**: All external services consolidated in `external_services` crate
- **Critical**: Config field renamed: `hyperswitch_client` → `user_client` in AppState
- **Critical**: Two modes: `mock_user_client` (local) or `hyperswitch` (production HTTP)
- **Critical**: Production requires `HYPERSWITCH_CLIENT_BASE_URL` and `HYPERSWITCH_CLIENT_INTERNAL_API_KEY` env vars
- **Pattern**: Use `UserInterface` trait for all user operations, not direct HyperSwitch calls
<!-- AI-UPDATED: 2025-01-07 - Secret management consolidation -->
- **Critical**: Use `external_services::secrets::*` imports, not root-level
- **Critical**: Must specify features in Cargo.toml (e.g., `features = ["no_encryption"]`)
- **Pattern**: Secret management uses state containers (SecuredSecret → RawSecret)

## Secret Management Patterns
<!-- AI-UPDATED: 2025-01-07 - Secret<T> wrapper implementation -->
- **Critical**: All secrets wrapped in `Secret<T>` type for safety
- **Critical**: Use `.peek()` to access secret values, never direct field access
- **Critical**: Debug output shows `*** String ***` instead of actual secret values
- **Critical**: Config secrets (jwt_secret, admin_api_key, internal_api_key, db_password) all use Secret<T>
- **Pattern**: Secret transformers must wrap results in Secret::new()
- **Pattern**: Secrets are serde transparent - deserialize to inner type automatically

## Application Encryption System
<!-- AI-UPDATED: 2025-01-29 - Encryption system integration -->
- **Critical**: All sensitive data uses ApplicationEncryptionInterface for encryption
- **Critical**: Two strategies: LocalEncryption (master key) and KeymanagerService (external)
- **Critical**: NO FALLBACK between strategies - deterministic behavior only
- **Critical**: Must call setup_key_for_identifier() before encryption with KeymanagerService
- **Critical**: Uses Identifier::ReconMerchant(merchant_id) for per-merchant key isolation
- **Pattern**: Configuration follows secrets_management pattern with tagged enums
- **Pattern**: ApplicationEncryptionClient stored in AppState for global access

## Staging Entries Metadata Encryption
<!-- AI-UPDATED: 2026-05-20 - System-wide staging entries encryption + batch chunking -->
- **Critical**: ALL staging entry metadata encrypted using merchant-specific keys across entire system
- **Critical**: Database schema change: staging_entries.metadata column changed from JSONB to BYTEA
- **Critical**: merchant_id column added to staging_entries table - REQUIRED for encryption key isolation
- **Critical**: metadata columns now mandatory (NOT NULL) - empty metadata stored as empty bytea (\x)
- **Critical**: ALL transformations must use batch encryption pattern to avoid O(n) performance bottleneck
- **Critical**: Consistent v1:base64 storage format across LocalEncryption and KeymanagerService strategies
- **Critical**: Batch operations must validate all items belong to same merchant for security
- **Pattern**: 3-pass processing: validate → batch encrypt → create entities
- **Pattern**: Use `FxHashMap<String, serde_json::Value>` with uuid keys for batch encryption
- **Security**: Never mix merchants in batch operations - encryption keys are completely isolated

## Security Boundaries
- **Rule**: IDs are always generated server-side
- **Rule**: Never trust client-provided IDs
- **Rule**: Validate at Core layer, not just API layer
- **Rule**: Auth validation happens in route handlers
- **Rule**: Encryption keys are per-merchant isolated - different merchants cannot decrypt each other's data
