# Changelog

All notable changes to Hyperswitch Reconciliation will be documented here.

- - -

## 2026.11.12.0

### Features

- **api-key:** Add create, list and update endpoints ([#30](https://github.com/juspay/hyperswitch-reconciliation/pull/30)) ([`dacd217`](https://github.com/juspay/hyperswitch-reconciliation/commit/dacd2174577d91e14c6f0b6bc6fd9a48f1ce4206))

**Full Changelog:** [`2026.11.11.0...2026.11.12.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.11.0...2026.11.12.0)

- - -

## 2026.11.11.0

### Features

- **api-key:** Add key generation, hashing and hash key config ([#24](https://github.com/juspay/hyperswitch-reconciliation/pull/24)) ([`8bc8ac5`](https://github.com/juspay/hyperswitch-reconciliation/commit/8bc8ac5ec4f2d95b8fdd1dae43fe78659023aeaf))

**Full Changelog:** [`2026.11.06.1...2026.11.11.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.06.1...2026.11.11.0)

- - -

## 2026.11.06.1

### Features

- **health:** Add `deep_health` endpoint to check dependencies ([#31](https://github.com/juspay/hyperswitch-reconciliation/pull/31)) ([`196eb93`](https://github.com/juspay/hyperswitch-reconciliation/commit/196eb93101a0d02b4d5f934e26f47d54b3a8c08c))

### Bug Fixes

- **health:** Update health checks for error-stack compatibility ([#40](https://github.com/juspay/hyperswitch-reconciliation/pull/40)) ([`2612746`](https://github.com/juspay/hyperswitch-reconciliation/commit/2612746a4622327613881efafcdfaa10fb1a7851))
- **ingestion:** Remove adyen webhook secrets from response ([#34](https://github.com/juspay/hyperswitch-reconciliation/pull/34)) ([`9b50bd1`](https://github.com/juspay/hyperswitch-reconciliation/commit/9b50bd185a8eddb958e1e4a244f9f967bd687535))

### Miscellaneous Tasks

- **deps:** Bump vulnerable dependencies to their patched versions ([#32](https://github.com/juspay/hyperswitch-reconciliation/pull/32)) ([`4024329`](https://github.com/juspay/hyperswitch-reconciliation/commit/40243296be574ed4f9d6e9cb3fba66fb5a6904d9))

**Full Changelog:** [`2026.11.06.0...2026.11.06.1`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.06.0...2026.11.06.1)

- - -

## 2026.11.06.0

### Bug Fixes

- **linking:** Make entry linking fields non null ([#39](https://github.com/juspay/hyperswitch-reconciliation/pull/39)) ([`330597d`](https://github.com/juspay/hyperswitch-reconciliation/commit/330597d4a1b03ce626ac2d513a9ee7b340658443))

**Full Changelog:** [`2026.11.05.0...2026.11.06.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.05.0...2026.11.06.0)

- - -

## 2026.11.05.0

### Bug Fixes

- **ingestion:** Scope SFTP file access to the configured merchant ([#25](https://github.com/juspay/hyperswitch-reconciliation/pull/25)) ([`969b731`](https://github.com/juspay/hyperswitch-reconciliation/commit/969b73113f781186b2fecbb87f888864d512d168))
- **linking:** Use detailed statuses and defer linking backfill ([#35](https://github.com/juspay/hyperswitch-reconciliation/pull/35)) ([`67ff8dd`](https://github.com/juspay/hyperswitch-reconciliation/commit/67ff8dda5d7a26a61b9570ca4d378248e2a784d3))

**Full Changelog:** [`2026.11.03.0...2026.11.05.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.03.0...2026.11.05.0)

- - -

## 2026.11.03.0

### Miscellaneous Tasks

- **migrations:** Chunk entry linking backfill queries ([#28](https://github.com/juspay/hyperswitch-reconciliation/pull/28)) ([`8b1e59f`](https://github.com/juspay/hyperswitch-reconciliation/commit/8b1e59f38c833c922f4d07c17ca667cd64130f56))

**Full Changelog:** [`2026.11.02.0...2026.11.03.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.02.0...2026.11.03.0)

- - -

## 2026.11.02.0

### Features

- **api-key:** Setup `api_key` table and queries ([#15](https://github.com/juspay/hyperswitch-reconciliation/pull/15)) ([`e6a6d8b`](https://github.com/juspay/hyperswitch-reconciliation/commit/e6a6d8b90c7d00c8f9916d15b83eb496e9a6db84))
- **file-storage:** Add support for OCI Object Storage ([#2](https://github.com/juspay/hyperswitch-reconciliation/pull/2)) ([`27b7c9d`](https://github.com/juspay/hyperswitch-reconciliation/commit/27b7c9d8cd231ead9d571d8a4037f12fe57bc7a3))
- **linking:** Implement independent entry linking processor ([#6](https://github.com/juspay/hyperswitch-reconciliation/pull/6)) ([`fe200ce`](https://github.com/juspay/hyperswitch-reconciliation/commit/fe200ce5b40c66d00bb05014ffcdee71ac458c3e))
- **secrets:** Add support for OCI Vault secret management ([#4](https://github.com/juspay/hyperswitch-reconciliation/pull/4)) ([`fb3445a`](https://github.com/juspay/hyperswitch-reconciliation/commit/fb3445a6bdddc4f0363a8c5fbfa1f66971004e7e))

### Bug Fixes

- **auth:** Enforce existing permissions on `profile` and `search_group` APIs ([#20](https://github.com/juspay/hyperswitch-reconciliation/pull/20)) ([`b201985`](https://github.com/juspay/hyperswitch-reconciliation/commit/b2019856f0724341ecbbae6524de8133588735e8))
- **deps:** Update dependencies to reject compromised CAs ([#16](https://github.com/juspay/hyperswitch-reconciliation/pull/16)) ([`5d1dcbf`](https://github.com/juspay/hyperswitch-reconciliation/commit/5d1dcbfc5ba3acfdd64cb4697a08b6d10514675a))

### Refactors

- **recon:** Move database interfaces to `storage_impl` crate ([#17](https://github.com/juspay/hyperswitch-reconciliation/pull/17)) ([`de4aceb`](https://github.com/juspay/hyperswitch-reconciliation/commit/de4aceb7cad2dfdb3e34d97df1c8b763f9586917))

**Full Changelog:** [`2026.10.07.0...2026.11.02.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.10.07.0...2026.11.02.0)

- - -
