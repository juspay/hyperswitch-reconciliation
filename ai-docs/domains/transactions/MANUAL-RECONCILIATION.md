# Manual Reconciliation (Exception Management)

> Read with `AGENT-NOTES.md`. Covers both versions of the human-driven reconcile endpoint:
> `POST /exception_management/transactions/{id}/manual_reconciliation` (v1) and `.../manual_reconciliation/v2`.
> Core: `recon/src/core/exception_management.rs`. Entry builders: `recon/src/domain/exception_management/manual_reconciliation.rs`.
> Validations: `recon/src/utils/exception_management.rs` (`EntryOpsVecExt` over the ops, `PreparedEntryVecExt` over the built entries).

## Why v2 exists
<!-- AI-UPDATED: 2026-09-22 - Delta reconciliation endpoint -->
- **v1's payload has no entry id.** `TransactionEntryWithData` carries no `entry_id`, so the server cannot tell an edited entry from an untouched one: it archives every entry and rebuilds the whole set from whatever the client sent. With `MAX_ENTRIES_PER_TRANSACTION` at 25 000 (`recon/src/domain/transaction.rs`) the FE would have to hold the full set in memory and ship it back on every call, while its own entry list APIs page at 50.
- **v2 takes operations on individual entries instead.** Untouched entries never appear on the wire; the server re-creates them itself from the rows it already read. `{id}` is the per-version PK, so the request names one specific version.
- **v1 stays registered and none of the fixes below were backported** — an FE on v1 still gets v1's behaviour. It is slated for deprecation (a `TODO` in `manual_transaction_reconciliation`) but not marked deprecated in code or OpenAPI. One visible change did reach it: `InvalidEntryStatus` now names an entry, v1 has no entry id to give, so its 400 message reads `with id: ""`.

## Three things v1 silently loses, and v2 does not
- **`exposed_metadata` is written as `json!({})`** on every entry v1 rebuilds. That field is what entry list filters and reporting read, so a manually reconciled transaction drops out of metadata-filtered views. v2 projects it for real (see the fresh/carried rule below).
- **Re-sent entries lose `primary_staging_entry_id` and `transformation_config_id`.** v1 filters already-`Processed` staging entries out of its lookup map, so both resolve to `None`.
- **…which strands their `search_duplicate` rows as `Archived`.** `EntryNew::batch_insert_chunked` reactivates a duplicate *only* for entries that carry a `primary_staging_entry_id` (`diesel_models/src/query/entry.rs`), so an entry that lost the field never comes back to `Active`. Dropping that field anywhere in a versioning flow has this effect — it is not specific to manual recon.
- **Neither v1 nor `force_reconcile`/void/post re-emits `search_entry_key` rows**, so searchable metadata kept pointing at the superseded version. v2 writes them for the new version (see below).

## The operations (`EntryOp`, internally tagged on `op`)
| Op | Effect |
|---|---|
| `update_entry` | apply `EntryOverrides` to an existing entry |
| `mark_received` | an `Expected` entry arrived → `Pending`; target must currently be `Expected` |
| `create_entry` | add a `NewEntry::Direct` or `NewEntry::Expected` |
| `create_with_staging_entry` | pull an unreconciled staging entry in as a new entry |
| `replace_with_staging_entry` | drop the named entry *and* add the staging-sourced one; net count delta zero |
| `remove_entry` | drop an entry |

- **The account is fixed; the amount is not.** `EntryOverrides` (`update_entry`, `mark_received`) can change `amount`, `entry_type`, `effective_at`, `metadata`, `order_id` and `transformation_id` on any existing entry, staging-sourced ones included, but has no `account_id`: v1 moves an entry by resending it on another account, and v2 dropped that in review. New entries name their account; staging-sourced entries take the staging row's.
- **Unknown keys are dropped, not rejected** (no `deny_unknown_fields`). An `account_id` in `EntryOverrides`, an `amount` in `StagingEntryOverrides` or a `transformation_id` on `kind: expected` is ignored, and the call still succeeds.
- **An op's `staging_entry_id` is the staging row's primary `id`** (per version, `GetStagingEntryResponse.id`), not its logical `staging_entry_id` — `prepare_staging_links` filters on `id`, as v1 does.
- Accepted transaction statuses: the nine in `MANUALLY_RECONCILABLE_STATUSES`. Anything else is a 400 `InvalidTransactionStatus`. v1 hand-lists the same nine in a `match`, so change both together.
- Only `mark_received` checks the target entry's status. `update_entry`, `remove_entry` and `replace_with_staging_entry` take the entry in whatever status the loaded version holds; the reasoning given in review is that entry statuses follow the transaction's, which is already restricted to those nine.
- **Payload validation is pure** (`EntryOpsVecExt::validate_payload`, no I/O) and runs before any entry is loaded: non-empty ops (`EmptyEntryOps`), every `entry_id`/`staging_entry_id` named at most once across the whole request (`DuplicateEntryOpTarget`), the entry cap, and amounts. `replace_with_staging_entry` contributes **both** of its ids to the duplicate check.
  - **The cap is computed from the transaction row**: its `entry_ids`, minus `remove_entry` targets, plus one per `create_entry`/`create_with_staging_entry`, against `MAX_ENTRIES_PER_TRANSACTION` → 400 `TooManyEntries`. So an oversized request fails before the entry load and before any encrypt or decrypt.
  - **Amounts are checked in minor units of the transaction currency**, for `create_entry` and `EntryOverrides.amount` alike. The float→minor conversion truncates, so `0.001` USD is 0 minor units and a 400 `InvalidAmount`, even though it is positive as a float.
- **The resulting entry set must still cover every account on the transaction's rule** (`PreparedEntryVecExt::validate_rule_account_coverage`, against `rule.strategy.get_recon_engine_type().get_rule_accounts()`) → 400 `MissingRuleAccountEntries` listing what is missing. This is what stops `remove_entry` from emptying a transaction: a zero-entry version is trivially balanced with no `Expected` entries, so without the check it would be written as `Matched(Manual)`, with `new_versioned` falling back to `now()` for its `effective_at`. Note a rule that *gains* an account after a transaction was created makes that transaction fail this check until an entry for the new account is added — which the endpoint itself can do with `create_entry`.
- The rule is fetched **before** the write (it is also what supplies `aging_config` for the response), so a missing rule is a 404 instead of a failure after a successful commit.
- Resulting status: any surviving `Expected` entry, or credits ≠ debits → `PartiallyReconciled`. Otherwise `Matched(Manual)` — and **every** entry in the new version, carried-forward ones included, is rewritten to `EntryStatus::Matched`.

## The rule that governs metadata: fresh vs carried
`exposed_metadata` is the entry's metadata projected through its transformation config's schema, and the `search_entry_key` rows are derived from that projection. Both are recomputed **exactly when the entry is new or an op supplies `metadata` or `transformation_id` for it**, never otherwise — a schema edit alone does not refresh a carried entry:

| case | metadata ciphertext | `exposed_metadata` | search keys |
|---|---|---|---|
| untouched entry, or an op supplying neither `metadata` nor `transformation_id` | copied | copied | copied from the rows fetched for the old entry |
| op supplies `metadata` | re-encrypted | re-projected | re-derived |
| op supplies `transformation_id` only | copied (decrypted only to project) | re-projected | re-derived |
| `create_entry` / staging-sourced entry | encrypted from the request, or the staging entry's ciphertext reused | projected, or the staging entry's copied | derived |

- **An untouched entry is never decrypted.** Every decrypt is bounded by the request: one per op that supplies `transformation_id` without `metadata`, and one per linked staging entry (`convert_back` in `prepare_staging_links`, which `build_search_duplicate` needs). Representing an entry as `DomainEntry` forces a decrypt, because `EncryptableValue` holds plaintext *and* ciphertext — that is why this flow works on `diesel_models::EntryNew` and not the domain type. At 25 000 entries a decrypt-per-entry pass is 25 000 key-manager round trips.
- **`PreparedEntry::projected` derives the search keys from the entry's own `exposed_metadata`**, so the two cannot disagree; `PreparedEntry::carried` is the copy path. Anything that builds an entry goes through one of the two.
- An entry with no transformation config projects through `NO_EXPOSED_KEYS` (`common_types::metadata_schema`) — i.e. `{}` — which is also what `create_entry` with `kind: expected` gets.

## Why the existing search keys are fetched
`load_existing_entries_with_search_keys` reads the version's entries **and** their `search_entry_key` rows into one `HashMap<entry_id, ExistingEntry>` that everything downstream uses.
- **It bounds the config/schema fetch by the request, not by the transaction.** Untouched entries carry their keys over verbatim, so their transformation configs are never needed. Re-deriving instead would require the `ExposedKeys` of every config referenced across all 25 000 entries, and a config that has since been deleted would fail the whole call.
- Updates and receipts fetch configs only when `metadata` or `transformation_id` is supplied.
- The fetch is `list_search_entry_keys_by_entry_ids_and_profile_id` with no limit, which rides `idx_sek_entry_id`. There is **no index on `search_entry_key.primary_transaction_id`**, so fetching a version's keys by that single column would seq-scan until a migration adds one. It takes the ids by value and logs only their count on error; logging the ids would put up to 25 000 of them in one line.
- Consequence to keep in mind: if a schema's `searchable` list changed after those entries were written, a carried-forward entry keeps the keys the old schema produced. That is consistent with its copied `exposed_metadata`.

## Overriding `transformation_id` forces a decrypt
`EntryOverrides.transformation_id` re-classifies an entry under a different config. The stored `exposed_metadata` was projected through the *old* schema and cannot be reused, and it cannot be re-projected from itself either — the new schema may expose keys the old projection dropped. So that entry's metadata is decrypted (`EncryptionHandler::decrypt_json_value`) purely to re-project; the ciphertext is stored back unchanged. This is bounded by the ops in the request, never by the entry count.
- **Supplying the id the entry already has still re-projects, on purpose.** The trigger is "the op supplied a `transformation_id`", not "the id changed", because a config's `metadata_schema_id` can be repointed and the schema itself edited — so the same config id can mean a different projection than the one stored. Sending the current id is the way to ask for a refresh.
- **The config must belong to the entry's account** → 400 `TransformationConfigAccountMismatch`.
  `EntryOpsVecExt::transformations_to_validate` checks config overrides against the existing entry's
  immutable account, and checks `create_entry`'s config against its requested account.
- A config id that does not resolve in the profile is a **404** `TransformationConfigNotFound` (RE_619). That variant was a 400 before v2, and the change also reaches `POST /transactions`, its only other raiser (`exposed_keys_for_entry`).
- **`StagingEntryOverrides` has only `effective_at`, `metadata` and `order_id`.** A link takes the account, `entry_type`, amount and `transformation_id` from the staging row, so the new entry starts out as what was ingested. Nothing keeps it that way: `EntryOverrides` allows amount, `entry_type` and `transformation_id`, so a later request can `update_entry` the linked entry (not the same request — its id is generated server-side). The staging row stays `Processed` for its full amount, and nothing flags that the entry no longer matches it. Only the account can never diverge.

## `search_entry_key` conventions
- `primary_transaction_id` is the **version PK** (`transaction.id`), not the logical `transaction_id` — same as `TransactionCreationData::get_new_transaction_data` uses on the creation path. Keys are therefore minted *after* the new transaction row is built, inside the same DB transaction.
- Rows are never deleted anywhere in the codebase. Superseded versions keep theirs; queries scope by `entry_id`, and archived entries' ids simply stop being current.

## Structure and cost
- Ops are indexed once by target entry id (`HashMap<&str, &EntryOp>`) and the entry set is walked once. Searching the entry vec per op instead is O(entries × ops), and `Vec::retain` per removal makes it worse — that shape was removed deliberately.
- The entry set is loaded whole and every entry is rewritten, exactly like `force_reconcile`, void, post and the engine flows. The O(N) write amplification is inherent to the versioning model and is not solved here; this endpoint solves the API contract only.
- **Entry order in a new version is undefined.** `Entry::list_by_transaction_id_and_version` has no `ORDER BY`, and v2 additionally iterates a `HashMap` under `buffer_unordered`. Do not write anything that depends on `entry_ids` ordering.
- Configs and schemas for linked staging entries are fetched twice — once in `prepare_staging_links` (which needs the full rows for `build_search_duplicate`) and once in `MetadataProjection::resolve`.
- **Crypto is per op, not batched.** Each encrypt, decrypt and staging `convert_back` is its own call (a key-manager round trip under `KeymanagerService`), `PREPARATION_MAX_CONCURRENT` (5) at a time. That is fine for FE-sized requests, but it is not the batch pattern CRITICAL-PATHS asks of transformations, and only the entry cap bounds the op count.

## Known gaps (intentional or deferred)
- **No concurrency control.** There is no row lock and no 409: if the version moves between the client reading it and saving, the write proceeds against the stale snapshot, and a losing `UNIQUE (transaction_id, version)` race surfaces as a 500. A live version is also mutable in place — `Transaction::append_expected_source_entry` rewrites `entry_ids` and the aggregates at the *same* version — so "version unchanged" does not mean "nothing changed". Two requests linking the same staging entry race the same way: `uniq_idx_staging_entry_id_version` makes the loser a 500, not a double link.
- **`search_expectation`s are archived and never recreated** on any manual path. A transaction a human has edited leaves the recon engine's automatic flow by design; late arrivals land in `NeedsManualReview`.
- **`remove_entry` and `replace_with_staging_entry` do not release the dropped entry's staging entry**: it stays `Processed` and its `search_duplicate` stays `Archived`.
- Validation errors reuse `EntryNotFound` (404) and `InvalidEntryStatus` (400) rather than v2-specific variants. The five variants v2 added — `TooManyEntries`, `EmptyEntryOps`, `DuplicateEntryOpTarget`, `TransformationConfigAccountMismatch`, `MissingRuleAccountEntries` — are RE_620–624, all 400.
