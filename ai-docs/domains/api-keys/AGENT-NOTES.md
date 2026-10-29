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
- `common_utils::api_key`: `PlaintextApiKey::generate(env_prefix)` makes `{env}_{64 base62}` (e.g. `dev_…`, `sbx_…`, `prd_…`) from `rand::thread_rng` (a CSPRNG), not nanoid. There is deliberately no `recon_` marker: recon is its own service. `display_prefix()` is `{env}_` plus the first 6 random chars. `PlaintextApiKey::hash(&ApiKeyHashKey)` is the **only** hash function, so creation and authentication can't diverge. Request keys go through `from_request`, which trims.
- `auth.api_key_hash_key` (64 hex chars, 32 bytes) is `Option`. Unset = `AppState.api_key_hash_key` is `None` = feature off. There is deliberately no default value in `AuthConfig::default()`. `config/prod.toml` has it commented out until the secret exists.
- The env prefix comes from `recon::env::prefix_for_env()` (`Env::prefix`): `dev`, `sbx` (`RUN_ENV=sandbox`) or `prd`. There is no separate config field. Locally, `RUN_ENV=sandbox` loads `config/prod.toml` (see `config_path`; there is no `sandbox.toml`). Deployed sandbox and prod get their values from `RECON__AUTH__API_KEY_HASH_KEY` (or KMS), not from the file. They must use different hash keys and databases: that, not the prefix, is what stops a sandbox key working in prod. The prefix only labels the environment.
- The hash key is parsed in `initialize_server` **after** decryption, not in `AuthConfig::validate` (ciphertext under KMS, see `patterns/CONFIGURATION.md`). A malformed value fails startup. Parse errors never include the value.
- **No rotation**: changing the hash key invalidates every issued key.
- The row `id` is not generated in `common_utils::api_key`. The create handler (management endpoints PR) calls `generate_id_with_default_len("apikey")`. The `id` is the safe handle for URLs and logs; never log the plaintext key or its hash.
