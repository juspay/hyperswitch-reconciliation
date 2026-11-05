# API Keys - Agent Notes

Merchant-level API keys for machine access to recon. The table, diesel model, store interface and key generation/hashing exist so far; there are no endpoints or auth yet.

## Data Model
- **`merchant_id` is not a database foreign key**: the repo does not enforce FKs at DB level (`domains/database`), so callers must check the merchant exists.
- **Only a keyed hash is stored** in `hashed_api_key`, globally `UNIQUE` across merchants. A lookup by hash will not involve the merchant, so the caller must compare the key's merchant with the request's.
- **`hashed_api_key` is `Secret<String>`**: `generic_insert` attaches the `Debug` of the inserted values to its error. Masking keeps the hash out of logs when an insert fails.
- **`ApiKey` is deliberately not `Serialize`**: it can't be returned from an endpoint by accident. Build a response DTO.
- **`created_by` is nullable** (`Option<String>` in the model) so a non-user creator, e.g. admin API, can be added later without an `ALTER TABLE`. For now key creation is JWT-only, so the core layer must always set it to the `user_id`.
- **`expires_at` NULL means never expires.** Whether that is allowed is a core-layer rule, so the column stays nullable. Expiry is not checked in the store.
- **`expires_at` must not change after creation**: moving it would let a leaked or old key be extended indefinitely. To get a longer life, create a new key and revoke the old one.

## Status
- `status` is `VARCHAR(64)`, not a DB enum, so adding a state needs no `ALTER TYPE`. Expiry is derived from `expires_at`, never stored as a status.
- Find and list only return `status = 'active'` rows. Nothing writes `revoked` yet.

## Store
- Queries are scoped by `merchant_id`, so one merchant's id can never read another's key. Find returns `NotFound`; list returns an empty `Vec`.

## Key Generation and Hashing
- `common_utils::api_key`: `PlaintextApiKey::generate(env_prefix)` makes `{env}_{64 base62}` (e.g. `dev_…`, `sbx_…`, `prd_…`) from `crypto::generate_cryptographically_secure_random_string` (OS CSPRNG via `OsRng`), not nanoid. There is deliberately no `recon_` marker: recon is its own service. `display_prefix()` is `{env}_` plus the first 6 random chars. `PlaintextApiKey::hash(&ApiKeyHashKey)` is the **only** hash function, so creation and authentication can't diverge. It returns `HashedApiKey(Secret<String>)`, so the hash is masked in `Debug` like the `hashed_api_key` column; use `peek()` or `Into<Secret<String>>` to store it. Request keys go through `from_request`, which trims. `ApiKeyHashKey` and `PlaintextApiKey` wrap `StrongSecret` (zeroized on drop, masked `Debug`); the hash key is decoded straight into a `[u8; 32]` so no unzeroized `Vec` is left behind.
- `api_keys.hash_key` (own `[api_keys]` section, not `[auth]`; 64 hex chars, 32 bytes) is `Option`. Unset = `AppState.api_key_hash_key` is `None` = feature off and the app still starts (prod boots without it until its secret is provisioned). Set but malformed fails startup. When `None`, authenticating with an API key must fail closed with the same generic `401` as an invalid key (log server-side; never hash with a fallback key, never fall through to another auth path), and key-management endpoints return a clear "API keys not enabled" error. Handlers should reach the key through one `AppState` accessor that returns that error, not match on the `Option`. There is deliberately no default value in `ApiKeysConfig`. `config/prod.toml` has it commented out until the prod secret exists; `config/sandbox.toml` has it enabled (sandbox releases first), so the sandbox secret must be provisioned before deploying or startup fails on the unresolved placeholder.
- The env prefix comes from `recon::env::prefix_for_env()` (`Env::prefix`): `dev`, `sbx` (`RUN_ENV=sandbox`) or `prd`. There is no separate config field. `RUN_ENV=sandbox` loads its own `config/sandbox.toml` (see `config_path`), a copy of `prod.toml` with sandbox-named DB/bucket values, so a missed override can't reach production resources. Keep the two in sync when adding settings. Deployed sandbox and prod get their values from `RECON__API_KEYS__HASH_KEY` (or KMS), not from the file. They must use different hash keys and databases: that, not the prefix, is what stops a sandbox key working in prod. The prefix only labels the environment.
- The hash key is parsed in `initialize_server` **after** decryption, not in `Config::validate` (ciphertext under KMS, see `patterns/CONFIGURATION.md`). A malformed value fails startup. Parse errors never include the value.
- **No rotation**: changing the hash key invalidates every issued key.
- The row `id` is not generated in `common_utils::api_key`. The create handler (management endpoints PR) calls `generate_id_with_default_len("apikey")`. The `id` is the safe handle for URLs and logs; never log the plaintext key or its hash.
