# API Keys - Agent Notes

Merchant-level API keys for machine access to recon. This file records what is implemented and the non-obvious why.

## Implemented
- `api_key` table, diesel model (`ApiKey`, `ApiKeyNew`, `ApiKeyUpdate`) and the store interface `ApiKeyInterface` on `Store`.

## Data Model
- **Table `api_key`, PK `id`**: singular table name per repo convention (`account`, `business_profile`).
- **`merchant_id` is not a database foreign key**: the repo does not enforce FKs at DB level (`domains/database`), so callers must check the merchant exists.
- **Only a keyed hash is stored** in `hashed_api_key`, globally `UNIQUE`. The lookup is one index hit and does not involve the merchant, so the caller must compare the key's merchant with the request's.
- **`hashed_api_key` is `Secret<String>`**: `generic_insert` attaches the `Debug` of the inserted values to its error. Masking keeps the hash out of logs when an insert fails.
- **`ApiKey` is deliberately not `Serialize`**: it can't be returned from an endpoint by accident. Build a response DTO.
- **`prefix`** holds the first characters of the plaintext key, for display only.
- **`created_by`** is the `user_id` of whoever minted the key. It is `NOT NULL`, so a key always has a known creator.
- **`expires_at` NULL means never expires.** Whether that is allowed is a core-layer rule, so the column stays nullable. Expiry is not checked in the store.
- **`expires_at` is fixed at creation**: `ApiKeyUpdate::Update` only changes `name` and `description`. Letting an update move the expiry would let a leaked or old key be extended indefinitely; to get a longer life, create a new key and revoke the old one.

## Revoking
- Revoke = `ApiKeyUpdate::Revoke`, which sets `status = revoked` and bumps `last_modified_at`. `status` is the `ApiKeyStatus` DB enum (`active`, `revoked`). Expiry is derived from `expires_at`, never stored as a status.
- Every find, list and update filters `status = 'active'`. A revoked key is invisible, **cannot be updated or revived**, and revoking twice is `NotFound`.
- Because every update predicate includes `status = 'active'`, a concurrent revoke and update cannot both succeed, so rows need no version column for optimistic locking.

## Updates
- `ApiKeyUpdate::Update` uses `Option<Option<String>>` for the nullable `description`: `None` = leave alone, `Some(None)` = set to NULL.
- All update and find-by-id queries are scoped by `merchant_id`, so one merchant's id can never touch another's key (`NotFound`).

## Testing
- Store-level tests: `crates/recon/tests/api/api_key_store_test.rs`, using `common::test_store()` (a pool on the test database, no HTTP server). Use a fresh `MerchantId::generate()` per test; tests share one database.
