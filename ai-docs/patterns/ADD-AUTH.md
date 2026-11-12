# Pattern: Add Authentication

## Credentials
Credentials are in the `Authorization` header, parsed once by `Credential::from_headers`
(same format as Hyperswitch v2):

- `Authorization: Bearer <jwt>`
- `Authorization: admin-api-key=<key>`

Exactly one header; unknown scheme, empty value or a second header → 401. Don't add new
credential headers. Only exception: `x-internal-api-key` for service-to-service calls.

## Validators
User-facing routes use one builder, `UserAuth`. It returns
`AuthContext { merchant_id, profile_id, actor }`: what the credential proved, not what the route needs. Don't add per-route context types or
`JwtOrX` validators; a merchant-level route ignores `profile_id`.

```rust
UserAuth::jwt(Permission::ProfileReconTransactionRead)  // JWT + permission check
UserAuth::jwt(Permission::ProfileAccountRead)
    .require_profile_id(path_profile_id)                // path id must match the JWT
UserAuth::jwt_without_permission()                      // any user of the JWT's merchant/profile
    .require_merchant_id(path_merchant_id)

// No context
AdminApiAuth        // admin key
InternalApiKeyAuth  // x-internal-api-key
NoAuth
```

Route usage:
```rust
api::server_wrap(
    UserAuth::jwt(Permission::ProfileReconTransactionRead),
    &http_req,
    state,
    |state, auth| async move { core::transaction::list_transactions(&state, auth, query).await },
)
.await
```

- `UserAuth::validate` matches on the credential that arrived. A credential the route doesn't accept
  (e.g. an admin key on a `UserAuth` route) → 401.
- `require_profile_id` / `require_merchant_id`: the path id must equal the authenticated one, else 401.
- `jwt(P)` checks `P` via Hyperswitch `authorize_user_token`; `jwt_without_permission()` skips it.
- There is no separate profile/merchant validator: the permission already says its level
  (`entities` in `generate_permissions!`), and both levels authenticate the same way.

## Actor
`auth.actor` is `Actor::User { user_id }`. For `modified_by`/`created_by` use `auth.user_id()`
(`Option`): those columns are resolved back to Hyperswitch users, so a non-user id there makes
transaction listing fail.

## Adding a credential kind
Add a `Credential` variant and an `Actor` variant. `UserAuth::validate` matches on `Credential` without
`_`, so the compiler stops at it until the new kind's handling is decided; give `UserAuth` one optional
rule field for it (at most one rule per credential kind) and a builder method to set it.

## OpenAPI
`Bearer` = JWT, `AdminApiKey` = admin key, `InternalApiKey` = internal key.
A route accepting several auth types lists each: `security(("Bearer" = []), ("AdminApiKey" = []))`.

## Internal API key
- `x-internal-api-key`, compared with `auth.internal_api_key`; `InternalApiKeyAuth` returns `()`.
- Only for service-to-service calls (e.g. ingestion → transformation loopback).
- Outside `server_wrap` there is no shared helper; compare inline with `get_header_value_by_key`
  (see `authenticate_sftp_internal`) and reject an empty configured key.

## Common mistakes
- Using `AdminApiAuth` for user-scoped operations (use `UserAuth`).
- Reusing the admin key for service-to-service calls.
- Leaving a system-triggered route on `NoAuth` when the caller can send the internal key.
- Writing a non-user id into `modified_by`.
- Forgetting the OpenAPI `security(...)` entry.

## Adding a Permission

Permissions are declared once in `crates/common_types/src/permissions.rs` via `generate_permissions!`:

```rust
Account: { scopes: [Read, Write], entities: [Profile, Merchant] },
```

- The macro generates `Permission::{Entity}{Resource}{Scope}` variants, e.g. `ProfileAccountRead` and `MerchantAccountRead`. `entities` is `Profile` or `Merchant` (`crates/recon_derive/src/macros/generate_permissions.rs`).
- **The entity decides who can hold the permission.** Recon has no roles; it asks Hyperswitch `/authorize`, which only lets a role whose entity level is at least the permission's entity hold it. `Merchant` excludes profile-level roles. Copying `entities: [Profile]` by mistake silently lets profile-level users in.
- The resource must also exist in Hyperswitch (`Resource`, permission groups and role grants), and that build must be deployed. Otherwise `authorize_token` rejects the string. The mock user client in tests allows everything, so tests can't prove this part.
- Use it with `JwtAuth::basic(Permission::MerchantAccountWrite)`.

## Related Patterns
- [ADD-ENDPOINT](ADD-ENDPOINT.md)
- [ERROR-HANDLING](ERROR-HANDLING.md)
