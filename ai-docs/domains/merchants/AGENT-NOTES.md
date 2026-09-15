# Merchants - Agent Notes

## Critical Rules
- Merchant IDs MUST use format "merchant_{nanoid(10)}" for auto-generated IDs
- Exception: `/auth/bootstrap_with_ids` accepts client-provided merchant IDs for Hyperswitch imports
- Merchant names cannot be empty (validated in core, not DB)
- Merchants cannot be deleted if they have business profiles (FK constraint)
- No update endpoint exists for merchants currently (immutable after creation)

## Common Gotchas
<!-- AI-UPDATED: 2025-01-07 - Admin API key authentication changes -->
- Merchant creation NOW REQUIRES admin API key (changed from NoAuth)
- Bootstrap endpoint NOW REQUIRES admin API key (changed from NoAuth)
- Token generation NOW REQUIRES admin API key (changed from NoAuth)
- But merchant retrieval DOES require matching JWT merchant_id
- Empty string merchant names pass DB constraint but fail business validation
- Merchant ID in JWT must exactly match URL path parameter or 401
<!-- AI-UPDATED: 2025-01-08 - New bootstrap_with_ids endpoint gotchas -->
- `/auth/bootstrap_with_ids` requires explicit conflict checking before DB insertion
- Empty merchant_id validation happens in core layer, returns 400 (not 500)
- Duplicate merchant_id returns 409 ResourceConflict (not DB constraint error)
- Client-provided IDs bypass nanoid generation but still require validation
<!-- AI-UPDATED: 2026-03-02 - MerchantId type-level validation migration -->
- **Validation Moved Toward Type Layer**: Merchant ID handling is increasingly enforced via typed `MerchantId` wrapping/generation instead of scattered string checks.
- **Auth Path Implication**: Empty-ID runtime checks in auth paths are no longer the primary guard; invalid IDs should fail earlier during typed conversion.
- **Call-Site Consistency Required**: Test helpers and service calls must pass merchant IDs through typed/validated flows to avoid endpoint-specific validation drift.

## Design Decisions
- We generate IDs server-side for security and uniqueness
  - Client-provided IDs could enable enumeration attacks
  - Nanoid provides URL-safe random IDs with ~119 bits entropy
- Merchants are mostly immutable
  - Only merchant_name could theoretically change
  - But no update endpoint implemented (business decision)
- No merchant deletion endpoint
  - Would require cascading through profiles → accounts → entries
  - Merchants don't have soft delete fields currently (no discarded_at)

## Relationships
- One merchant has many business profiles (1:N)
- Business profiles can have duplicate names within a merchant (no uniqueness constraint)
- Cannot query all merchants (no list endpoint) - security by design
- Must know merchant_id to access it (no discovery mechanism)

## When Working Here
- Always validate merchant_name is non-empty in core layer
- Check for existing merchant_id conflicts (though nanoid makes unlikely)
- Remember auth endpoints can create merchants without JWT
- But all other operations require JWT with matching merchant_id

## Authentication Patterns
<!-- AI-UPDATED: 2025-01-07 - Admin API key authentication -->
- `/auth/bootstrap` creates merchant + profile + JWT in one atomic operation (requires admin API key)
- `/auth/bootstrap_with_ids` creates merchant + profile with provided IDs + JWT (requires admin API key)
- `/auth/generate_token` requires existing merchant_id and profile_id (requires admin API key)
- POST `/merchant_accounts` requires admin API key (not JWT)
- GET `/merchant_accounts/{id}` requires JWT with matching merchant_id
- Admin API key uses `api-key` header, NOT `Authorization: Bearer` header
- Admin endpoints bypass JWT validation entirely - different auth flow
- No "super admin" concept for JWT - each JWT scoped to one merchant

## API Behavior
- POST `/merchant_accounts` returns 201 with created merchant
- GET `/merchant_accounts/{id}` returns 200 with merchant details
- POST `/auth/bootstrap_with_ids` returns 200 with merchant_id, profile_id, and JWT token
- No PUT/PATCH/DELETE endpoints implemented
- 404 if merchant doesn't exist (after auth check passes)
- 409 if merchant_id already exists (bootstrap_with_ids only)

## Encryption Integration
<!-- AI-UPDATED: 2025-01-29 - Merchant data encryption -->
- Merchant account creation now includes encryption of sensitive data
- Must call `state.encryption_client.setup_key_for_identifier(&identifier)` before encryption
- Uses `Identifier::ReconMerchant(merchant_id)` for per-merchant key isolation
- Encryption happens after merchant validation but before database insertion
- LocalEncryption strategy: Keys derived from master key + merchant ID
- KeymanagerService strategy: Creates key in external keymanager first
- Encryption failure prevents merchant account creation (atomic operation)

## Encryption Gotchas
<!-- AI-UPDATED: 2025-01-29 - Implementation details -->
- setup_key_for_identifier() must be called for EVERY merchant creation with keymanager
- Different merchant IDs produce completely different encryption keys
- LocalEncryption: Key derivation is deterministic and repeatable
- KeymanagerService: Key creation is idempotent - safe to call multiple times
- Encryption client uses the configured strategy (local_encryption vs key_manager)
- No fallback encryption - if configured strategy fails, merchant creation fails

## Testing Gotchas
- Integration tests use TestApp which auto-creates test database
- Each test runs in a transaction that's rolled back
- Can't test FK constraints in rolled-back transactions
- Bootstrap endpoint useful for setting up test data quickly
<!-- AI-UPDATED: 2025-01-29 - Encryption testing -->
- Tests must configure encryption strategy (usually local_encryption for speed)
- Mock keymanager service for KeymanagerService testing
- Verify encryption keys are different for different merchant IDs
- Test both successful encryption and encryption failure scenarios
