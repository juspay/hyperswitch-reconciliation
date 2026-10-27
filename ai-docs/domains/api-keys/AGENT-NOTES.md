# API Keys - Agent Notes

Merchant-level API keys for machine access to recon. This file records what is implemented and the non-obvious why.

## Implemented
- `api_key` table, diesel model (`ApiKey`, `ApiKeyNew`) and the store interface `ApiKeyInterface` on `Store`.

## Data Model
- **Table `api_key`, PK `id`**: singular table name per repo convention (`account`, `business_profile`).
- **`merchant_id` is not a database foreign key**: the repo does not enforce FKs at DB level (`domains/database`), so callers must check the merchant exists.
- **Only a keyed hash is stored** in `hashed_api_key`, globally `UNIQUE`. The lookup is one index hit and does not involve the merchant, so the caller must compare the key's merchant with the request's.
- **`hashed_api_key` is `Secret<String>`**: `generic_insert` attaches the `Debug` of the inserted values to its error. Masking keeps the hash out of logs when an insert fails.
- **`ApiKey` is deliberately not `Serialize`**: it can't be returned from an endpoint by accident. Build a response DTO.
- **`prefix`** holds the first characters of the plaintext key, for display only.
- **`created_by`** is the `user_id` of whoever minted the key. The column is nullable so a non-user creator (e.g. admin API) can be added later without an `ALTER TABLE`, but for now key creation is JWT-only, so the core layer must always set it.
- **`expires_at` NULL means never expires.** Whether that is allowed is a core-layer rule, so the column stays nullable. Expiry is not checked in the store.
- **`expires_at` is fixed at creation**: moving the expiry of an existing key would let a leaked or old key be extended indefinitely. To get a longer life, create a new key and revoke the old one.

## Status
- `status` is `VARCHAR(64)`, not a DB enum, so adding a state needs no `ALTER TYPE`. Expiry is derived from `expires_at`, never stored as a status.
- Find and list only return `status = 'active'` rows.

## Store
- `ApiKeyInterface` has insert, find by `(merchant_id, id)` and list by merchant (newest first). Queries are scoped by `merchant_id`, so one merchant's id can never read another's key (`NotFound`).
