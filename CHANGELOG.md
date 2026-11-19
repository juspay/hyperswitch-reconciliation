# Changelog

All notable changes to Hyperswitch Reconciliation will be documented here.

- - -

## 2026.11.17.1

### Features

- **linking:** Add profile-scoped linking processor status endpoint ([#51](https://github.com/juspay/hyperswitch-reconciliation/pull/51)) ([`5fa4bda`](https://github.com/juspay/hyperswitch-reconciliation/commit/5fa4bda315048a435ef14b4d2bab6e48d6ab43ae))

**Full Changelog:** [`2026.11.17.0...2026.11.17.1`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.17.0...2026.11.17.1)

- - -

## 2026.11.17.0

### Features

- **link-config:** Add name and description fields ([#50](https://github.com/juspay/hyperswitch-reconciliation/pull/50)) ([`cec5294`](https://github.com/juspay/hyperswitch-reconciliation/commit/cec52941db3bce21af013e2db379be0fd8e31560))

**Full Changelog:** [`2026.11.16.0...2026.11.17.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.16.0...2026.11.17.0)

- - -

## 2026.11.16.0

### Features

- **reporting:** Add endpoint to generate transaction-flows report ([#26](https://github.com/juspay/hyperswitch-reconciliation/pull/26)) ([`a768921`](https://github.com/juspay/hyperswitch-reconciliation/commit/a768921a97af64ad7d525fa8d3a4f48123fa577f))
- **staging-entry:**
  - Enable archived versions from v2 staging entry list ([#47](https://github.com/juspay/hyperswitch-reconciliation/pull/47)) ([`2894878`](https://github.com/juspay/hyperswitch-reconciliation/commit/28948780919ff0d0eec6f6163fb079306c0bd8b6))
  - Replace coarse statuses with granular ones ([#49](https://github.com/juspay/hyperswitch-reconciliation/pull/49)) ([`419a811`](https://github.com/juspay/hyperswitch-reconciliation/commit/419a8110b631da9121478aa27e627b961ce3ae90))
- **transaction:** Enable archived versions from the v2 transaction list ([#45](https://github.com/juspay/hyperswitch-reconciliation/pull/45)) ([`2b5769a`](https://github.com/juspay/hyperswitch-reconciliation/commit/2b5769ae998c081dc5dd17a4eb8ba0fc01b2740f))

### Bug Fixes

- **errors:** Give every error type its own code block ([#46](https://github.com/juspay/hyperswitch-reconciliation/pull/46)) ([`5317a08`](https://github.com/juspay/hyperswitch-reconciliation/commit/5317a0865ba54a381d19091ce0172d0f2bcbdb51))

**Full Changelog:** [`2026.11.13.1...2026.11.16.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.13.1...2026.11.16.0)

- - -

## 2026.11.13.1

### Bug Fixes

- **auth:** Add recon merchant API key permissions ([#44](https://github.com/juspay/hyperswitch-reconciliation/pull/44)) ([`aac1397`](https://github.com/juspay/hyperswitch-reconciliation/commit/aac13977e959dec5325ebcaa9461cb394918d37e))
- **imports:** Remove redundant self imports ([#43](https://github.com/juspay/hyperswitch-reconciliation/pull/43)) ([`3a9bcf6`](https://github.com/juspay/hyperswitch-reconciliation/commit/3a9bcf6cd399ae0427cc4b9e9c399a211bb527a5))

**Full Changelog:** [`2026.11.13.0...2026.11.13.1`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.13.0...2026.11.13.1)

- - -

## 2026.11.13.0

### Features

- **auth:** Add `merchant_api_key` auth validator ([#37](https://github.com/juspay/hyperswitch-reconciliation/pull/37)) ([`1b9919d`](https://github.com/juspay/hyperswitch-reconciliation/commit/1b9919d0c57028e00f7063767fd3f07011c7c0b1))

**Full Changelog:** [`2026.11.12.0...2026.11.13.0`](https://github.com/juspay/hyperswitch-reconciliation/compare/2026.11.12.0...2026.11.13.0)

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
