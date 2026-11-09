# Pattern: Add Authentication

## Credentials
Credentials are in the `Authorization` header, parsed once by `Credential::from_headers`
(same format as Hyperswitch v2):

- `Authorization: Bearer <jwt>`
- `Authorization: admin-api-key=<key>`

Exactly one header; unknown scheme, empty value or a second header → 401. Don't add new
credential headers. Only exception: `x-internal-api-key` for service-to-service calls.

## Validators
Pick the validator by what the handler needs, then add each accepted auth type with its own
permission. Never write `JwtOrX` validators.

```rust
// Profile-scoped -> AuthContext { merchant_id, profile_id, actor }
ProfileAuth::new().add_auth(AuthType::Jwt(Permission::ProfileReconTransactionRead))
ProfileAuth::new()
    .add_auth(AuthType::Jwt(Permission::ProfileAccountRead))
    .require_profile_id(path_profile_id)

// Merchant-scoped -> MerchantAuthContext { merchant_id, actor }
MerchantAuth::new()
    .add_auth(AuthType::JwtWithoutPermission)
    .require_merchant_id(path_merchant_id)

// No context
AdminApiAuth        // admin key
InternalApiKeyAuth  // x-internal-api-key
NoAuth
```

Route usage:
```rust
api::server_wrap(
    ProfileAuth::new().add_auth(AuthType::Jwt(Permission::ProfileReconTransactionRead)),
    &http_req,
    state,
    |state, auth| async move { core::transaction::list_transactions(&state, auth, query).await },
)
.await
```

- A credential whose auth type wasn't added → 401.
- `require_profile_id` / `require_merchant_id`: the path id must equal the authenticated one, else 401.
- `Jwt(P)` checks `P` via Hyperswitch `authorize_user_token`; `JwtWithoutPermission` skips it.

## Actor
`auth.actor` is `Actor::User { user_id }`. For `modified_by`/`created_by` use `auth.user_id()`
(`Option`): those columns are resolved back to Hyperswitch users, so a non-user id there makes
transaction listing fail.

## Adding an auth type
Add a `Credential` + `AuthType` variant and a match arm in each validator. Match arms are explicit
(no `_`), so the compiler lists every place to update.

## OpenAPI
`Bearer` = JWT, `AdminApiKey` = admin key, `InternalApiKey` = internal key.
A route accepting several auth types lists each: `security(("Bearer" = []), ("AdminApiKey" = []))`.

## Internal API key
- `x-internal-api-key`, compared with `auth.internal_api_key`; `InternalApiKeyAuth` returns `()`.
- Only for service-to-service calls (e.g. ingestion → transformation loopback).
- Outside `server_wrap` there is no shared helper; compare inline with `get_header_value_by_key`
  (see `authenticate_sftp_internal`) and reject an empty configured key.

## Common mistakes
- Using `AdminApiAuth` for user-scoped operations (use `ProfileAuth`/`MerchantAuth`).
- Reusing the admin key for service-to-service calls.
- Leaving a system-triggered route on `NoAuth` when the caller can send the internal key.
- Writing a non-user id into `modified_by`.
- Forgetting the OpenAPI `security(...)` entry.

## Related Patterns
- [ADD-ENDPOINT](ADD-ENDPOINT.md)
- [ERROR-HANDLING](ERROR-HANDLING.md)
