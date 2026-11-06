# API Keys - Agent Notes

Merchant-level API keys for machine access to recon. The table, store, key generation/hashing and the JWT-only management endpoints exist; the auth validators (`ApiKeyAuth`, `JwtOrApiKey`) exist but no route uses them yet.

## Data Model

- **`merchant_id` is not a database foreign key**: the repo does not enforce FKs at DB level (`domains/database`), so callers must check the merchant exists.
- **Only a keyed hash is stored** in `hashed_api_key`, globally `UNIQUE` across merchants. A lookup by hash will not involve the merchant, so the caller must compare the key's merchant with the request's.
- **`hashed_api_key` is `Secret<String>`**: `generic_insert` attaches the `Debug` of the inserted values to its error. Masking keeps the hash out of logs when an insert fails.
- **`ApiKey` is deliberately not `Serialize`**: it can't be returned from an endpoint by accident. Build a response DTO.
- **`created_by` is nullable** (`Option<String>` in the model) so a non-user creator, e.g. admin API, can be added later without an `ALTER TABLE`. For now key creation is JWT-only, so the core layer must always set it to the `user_id`.
- **`expires_at` is `NOT NULL`: there are no never-expiring keys.** The 1-year cap is a core-layer rule (`MAX_EXPIRY_DAYS` in `core/api_key.rs`), not a DB constraint. Expiry is not checked in the store.
- **`expires_at` must not change after creation**: moving it would let a leaked or old key be extended indefinitely. To get a longer life, create a new key and revoke the old one.

## Status

- `status` is `VARCHAR(64)`, not a DB enum, so adding a state needs no `ALTER TYPE`. Expiry is derived from `expires_at`, never stored as a status.
- Find and list only return `status = 'active'` rows. Revoke writes `revoked` and bumps `last_modified_at`; every update is filtered to `status = 'active'`, so a revoked key can't be revived or revoked twice (both return `NotFound`).

## Store

- Queries are scoped by `merchant_id`, so one merchant's id can never read another's key. Find returns `NotFound`; list returns an empty `Vec`.

## Key Generation and Hashing

- `common_utils::api_key`: `PlaintextApiKey::generate(env_prefix)` makes `{env}_{64 base62}` (e.g. `dev_…`, `sbx_…`, `prd_…`) from `crypto::generate_cryptographically_secure_random_string` (OS CSPRNG via `OsRng`), not nanoid. There is deliberately no `recon_` marker: recon is its own service. `display_prefix()` is `{env}_` plus the first 6 random chars. `PlaintextApiKey::hash(&ApiKeyHashKey)` is the **only** hash function, so creation and authentication can't diverge. It returns `HashedApiKey(Secret<String>)`, so the hash is masked in `Debug` like the `hashed_api_key` column; use `peek()` or `Into<Secret<String>>` to store it. Request keys go through `from_request`, which trims. `ApiKeyHashKey` and `PlaintextApiKey` wrap `StrongSecret` (zeroized on drop, masked `Debug`); the hash key is decoded straight into a `[u8; 32]` so no unzeroized `Vec` is left behind.
- `api_keys.hash_key` (own `[api_keys]` section, not `[auth]`; 64 hex chars, 32 bytes) is **required**: `AppState.api_key_hash_key` is a plain `ApiKeyHashKey`. A missing, empty or malformed value fails startup, so every environment (prod included) must have the secret provisioned before deploying a build with this code. There is deliberately no default value (`ApiKeysConfig::default()` is empty so a missing section fails `validate`). `config/prod.toml` and `config/sandbox.toml` reference `${API_KEY_HASH_KEY}`, which must be supplied (env override `RECON__API_KEYS__HASH_KEY` or KMS); an unresolved placeholder fails startup.
- The env prefix comes from `recon::env::prefix_for_env()` (`Env::prefix`): `dev`, `sbx` (`RUN_ENV=sandbox`) or `prd`. There is no separate config field. `RUN_ENV=sandbox` loads its own `config/sandbox.toml` (see `config_path`), a copy of `prod.toml` with sandbox-named DB/bucket values, so a missed override can't reach production resources. Keep the two in sync when adding settings. Deployed sandbox and prod get their values from `RECON__API_KEYS__HASH_KEY` (or KMS), not from the file. They must use different hash keys and databases: that, not the prefix, is what stops a sandbox key working in prod. The prefix only labels the environment.
- The hash key is format-checked in `initialize_server` **after** decryption, not in `Config::validate` (which only checks it is non-empty) (ciphertext under KMS, see `patterns/CONFIGURATION.md`). A malformed value fails startup. Parse errors never include the value.
- **No rotation**: changing the hash key invalidates every issued key.
- The row `id` is not generated in `common_utils::api_key`. The create handler (management endpoints PR) calls `generate_id_with_default_len("apikey")`. The `id` is the safe handle for URLs and logs; never log the plaintext key or its hash.

## Management Endpoints (`/api_keys`, JWT or admin key)

- POST/GET `/api_keys`, GET/PUT/DELETE `/api_keys/{api_key_id}`. DELETE is a soft revoke and returns the key with `status: revoked`; afterwards get/list/update/delete all return 404.
- **Permission is `MerchantReconApiKeyRead/Write`** (`entities: [Merchant]`, needed `Merchant` in `recon_derive`'s `EntityType`). Hyperswitch must know the `ReconApiKey` permission (a change in the Hyperswitch repo) or its `authorize_token` rejects the string. Only a merchant-level role can hold it, so profile-level users can't mint keys. The mock user client allows every permission, so tests can't prove this part.
- The merchant always comes from `AuthContext`; every store call filters by it, so another merchant's key id is a 404, not a 403.
- Create: `expires_at` must be stated: a future datetime, or `"never"` (stored as NULL, returned as `"never"`). Missing or `null` is a 400, so a client bug can't mint a permanent key. No maximum, matching Hyperswitch. Without `last_used_at`, permanent keys can't be audited for staleness. It is not updatable (`UpdateApiKeyRequest` has `deny_unknown_fields`, so sending it is a 400). Returns the plaintext once, with `Cache-Control: no-store` (`ApplicationResponse::JsonNoStore`).
- `created_by` is the JWT `user_id`.
- **Admin key access** (`JwtOrAdminApiAuth`): `Authorization: admin-api-key=<secret>` plus a mandatory `x-merchant-id`, as in Hyperswitch. No permission check, so it is the one way around the merchant-level-only rule. The merchant must exist (401 otherwise) and the header scopes every query, so it can only touch that merchant's keys. `created_by` is NULL. Exactly one credential is allowed (see Credentials).
- Datetimes in API models must use `common_utils::custom_serde::iso8601` (RFC 3339 with an offset, e.g. `2027-01-01T00:00:00Z`). Plain `PrimitiveDateTime` serde is not ISO: it rejected real clients while tests that reused the same serde passed, so tests send the string form.

## Credentials (`services/auth.rs`)

- **One `Authorization` header carries the credential** (the Hyperswitch v2 convention): `Bearer <jwt>`, `api-key=<key>`, or `admin-api-key=<secret>`. `extract_credential` is the only parser and requires exactly one credential; none, two (including two `Authorization` headers), an unknown scheme, or an empty value are all 401. Anything else in the value (a comma, spaces) is not parsed: it is hashed or compared as part of the secret, so it fails there.
- **The old `api-key` header is gone**: no client used it, so it was removed rather than kept for compatibility. Sending it is a 401 (no credential). Admin routes (`AdminApiAuth`) now need `Authorization: admin-api-key=<secret>` too.
- **`ApiKeyAuth`** flow: hash, `find_active_api_key_by_hash`, not expired, mandatory `x-profile-id`, merchant and profile exist and the profile belongs to the key's merchant (`validate_merchant_and_profile`, shared with `JwtAuth`). Every failure is the same 401; the log carries the key `id`, never a header value. The hash lookup is not merchant-scoped, so the merchant comparison is the isolation check.
- **`AuthContext.auth_method`** is `Jwt` or `ApiKey`. For keys `user_id` is `apikey:<id>`, which is not a Hyperswitch user: code that resolves `user_id` through Hyperswitch (`get_modified_by`) must branch on `auth_method` and omit `modified_by` for keys (no user lookup). That branch lands with the first route that enables keys.
