# Business Profiles - Agent Notes

## Critical Rules
- **Profile Hierarchy**: Business profiles belong to merchants (many-to-one)
- **Isolation Boundary**: Profiles provide data isolation within a merchant
- **No Cross-Profile Access**: Operations MUST be scoped to authenticated profile
- **Immutable Relations**: A profile's merchant_id cannot be changed
- **Profile ID Format**: Use `profile_{nanoid}` for auto-generated IDs
- **Exception**: `/auth/bootstrap_with_ids` accepts client-provided profile IDs for Hyperswitch imports

## Common Gotchas
- **Not Optional**: Unlike some systems, every operation requires a profile_id
- **No Default Profile**: Don't assume a "default" profile exists for merchants
- **Profile Switching**: Client must explicitly provide profile_id - no implicit selection
- **Deletion Cascades**: Deleting a profile would orphan accounts/entries - handle carefully
- **Name Uniqueness**: Profile names need not be unique even within a merchant
<!-- AI-UPDATED: 2025-01-08 - New bootstrap_with_ids endpoint gotchas -->
- `/auth/bootstrap_with_ids` requires explicit conflict checking for profile_id before DB insertion
- Empty profile_id validation happens in core layer, returns 400 (not 500)
- Duplicate profile_id returns 409 ResourceConflict (not DB constraint error)
- Client-provided profile IDs bypass nanoid generation but still require validation
<!-- AI-UPDATED: 2025-08-05 - Profile name encryption implementation -->
- Profile names are encrypted at DB level using merchant-specific encryption keys
- Must call `get_encryptor_for_merchant()` before any profile operations involving names
- Diesel → Domain conversion automatically decrypts profile names via `BusinessProfile::convert_back()`
- API responses use `EncryptableName` type which handles display/serialization securely

## Design Decisions
- **Why Business Profiles**:
  - Enables multiple businesses under one merchant account
  - Provides data isolation for different use cases
  - Allows separate reconciliation contexts
- **Why Not Hierarchical**:
  - Keeps permission model simple
  - Avoids complex inheritance rules
  - Clear ownership boundaries
- **Why Merchant Relationship**:
  - Billing aggregation at merchant level
  - Unified authentication/authorization
  - Shared configuration where appropriate

## Relationships
- **Merchant Account**: Many-to-one - profiles belong to exactly one merchant
- **Accounts**: One-to-many - accounts belong to exactly one profile
- **Transactions**: One-to-many - transactions scoped to profile
- **Auth Context**: Profile ID is part of every authenticated request

## When Working Here
- **Creating Profiles**:
  - Validate merchant exists and is active
  - Ensure authenticated user belongs to merchant
  - Generate unique profile_id with prefix
  - Get merchant encryptor before encrypting profile_name using `state.encryption_client.get_encryptor_for_merchant()`
- **Querying Data**:
  - ALWAYS include profile_id in WHERE clause
  - Never allow cross-profile data access
  - Validate profile belongs to authenticated merchant
  - Use `BusinessProfile::convert_back()` to decrypt profile names from diesel models
- **Profile Management**:
  - Consider implications of profile deletion
  - Implement proper cleanup for orphaned data
  - Maintain audit trail of profile operations
  - Remember profile names are encrypted - use domain models for business logic, not diesel models

## API Behavior
- **List Profiles**: Returns only profiles for authenticated merchant
- **Create Profile**: Auto-generates profile_id, requires merchant authentication
- **Get Profile**: Validates merchant ownership before returning
- **Update Profile**: Currently only supports name updates
- **Delete Profile**: Should be soft delete with careful cascade handling
- **Bootstrap Existing**: `/auth/bootstrap_with_ids` creates profile with provided profile_id (admin API key required)
- **Conflict Detection**: Returns 409 if profile_id already exists (bootstrap_with_ids only)

## Testing Gotchas
- **Multi-Profile Scenarios**: Test data isolation between profiles
- **Authorization Tests**: Verify cross-profile access is blocked
- **Merchant Validation**: Test profile operations with wrong merchant context
- **Concurrent Creation**: Test race conditions in profile creation
- **Orphaned Data**: Test cleanup when profiles are deleted
