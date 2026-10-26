# Entry Linking - Agent Notes

## Purpose and Link Semantics

The linking processor associates persisted entries from transaction and confirmation flows on the same account, using a separate `link_config`. It runs after entry creation and does not change reconciliation outcomes or account balances.

| Concept | Meaning |
|---|---|
| `EntryLinkStatus::Final` (`final`) | Final link between posted entries |
| `EntryLinkStatus::Provisional` (`provisional`) | Historical link between non-posted entries |
| `entry.linking_status` | Processing state: `pending`, `processed`, `failed`, or `not_applicable` |
| `search_entry_link_key.status` | Search state: `open` / `matched` for non-posted keys, `posted_open` / `posted_matched` for posted keys |

- **Expected placeholders start with `NotApplicable`; other newly created entries start with `Pending`.** The queue filters only on linking status, so this is constructor behavior, not an entry-kind guard in the processor. Explicit enqueue accepts only Failed linking status and rejects NotApplicable entries. Processing still requires transformation/schema resolution, a matching config, trigger, and usable identifier.
- **Historical links are intentional.** Archiving an entry preserves its linking fields and keys. Archived and void entries are not excluded from the pending query or from historical matching. Do not add a current-version-only filter or invalidate links merely because reconciliation replaced an entry.
- **Provisional does not mean a single latest row.** Non-posted matching retains history and creates links to all matching keys, including keys already marked `Matched`.
- **`Processed` does not mean linked.** An entry with no counterpart still gets a searchable key and becomes `Processed`; a later entry on the opposite side can find it.
- Link endpoints (`transaction_entry_id`, `confirmation_entry_id`) refer to physical `entry.id` values. They do not refer to logical transaction IDs or staging-entry IDs.

## Separation from Reconciliation

- `link_config` owns linking rules; `recon_rule.strategy` continues to own reconciliation. Linking uses `search_entry_link_key`, not `search_expectation` or `search_entry_key`.
- Linking's transaction/confirmation **side comes from the entry's transformation config -> metadata schema -> processing mode**. It is not inferred from debit/credit direction, source/target account role, or entry status.
- The owning transaction is resolved by logical transaction ID plus version. Its `profile_id` scopes config/schema resolution and its `rule_id` selects the appropriate side of the link config.
- Manual attachment of staging entries to a transaction is a separate feature; see [manual reconciliation](../transactions/MANUAL-RECONCILIATION.md).

## Config Selection and Triggers

1. Fetch active configs for the entry's account and the owning transaction's profile.
2. Match the transaction's rule ID against `transaction_rule_id` or `confirmation_rule_id`, according to the linking side.
3. Evaluate that side's trigger against the entry.
4. Select the highest `priority` among matching configs. Only one config is used; processing failure does not fall back to a lower-priority config.

- One config has **one shared account** and two rule references. Both sides must select the same config ID for their keys to match.
- Priorities are unique within each `(profile_id, account_id)`, including inactive configs. Different accounts or profiles may reuse a priority. Disabling a config does not release its priority.
- `LinkTrigger::V1` is an OR of AND groups (`any` containing groups of `all`). It differs from recon-rule `Trigger::V1`/`V2`; its operators are plain `equals` / `not_equals` strings.
- Missing fields make a condition false, including `not_equals`. Comparisons use extracted string values.
- Both `transaction_trigger` and `confirmation_trigger` must contain at least one `any` group, and every group must contain at least one `all` condition. Create and update reject empty arrays with HTTP 400 `LC_904`, including an empty group alongside valid groups. Errors identify the trigger field and, for empty `all`, the group index.
- Trigger evaluation itself uses OR/AND iterator semantics: an empty `any` matches nothing and an empty `all` matches everything. Direct storage insertion bypasses the API shape validation.

Example stored trigger value for either config side (the API omits `trigger_version`):

```json
{
  "trigger_version": "v1",
  "any": [
    {
      "all": [
        { "field": "metadata.event_type", "operator": "equals", "value": "payment" },
        { "field": "currency", "operator": "equals", "value": "USD" }
      ]
    }
  ]
}
```

## Config API and Update Constraints

- Config APIs use JWT profile scope: `GET /link_configs` and `GET /link_configs/{link_config_id}` require `ProfileReconRuleRead`; `POST /link_configs` and `PUT /link_configs/{link_config_id}` require `ProfileReconRuleWrite`. Listing includes inactive configs and sorts by ascending priority, then ID; processing selects only active configs and prefers the highest priority.
- **The account cannot be changed after creation.** `account_id` is absent from both the update request and Diesel changeset. Serde ignores unknown request fields, so sending it in a PUT does not move the config or produce a validation error. Create another config for a different account.
- `LinkConfigValidator::validate_with_db` validates the create account and both rules against the JWT profile. Updates validate supplied rules, triggers, identifiers, and priority; omitted fields are not revalidated. Blank IDs follow lookup and return 404; storage failures return 500.
- Profile ownership does not prove rule/account compatibility: validation does not check that either rule uses the config's account or has the intended processing side. Config authors must align these with the entries that will be processed.
- API triggers and identifiers omit the storage tags `trigger_version` and `link_search_version`; conversion adds the V1 wrappers. Empty composite keys return 400; duplicate fields are accepted. Trigger validation checks non-empty groups and conditions, without restricting condition values. API priority is `u8` (0-255), although responses/storage use `i16`.
- Priority validation checks all configs for the same profile/account and excludes the config being updated, so retaining its own priority is valid. A detected conflict returns HTTP 400 `LC_904`. The database unique index also enforces uniqueness; a write-time conflict after the precheck follows the storage-error path and returns 500.
- **A multi-field PUT is not atomic.** Validation runs first, then separate writes apply rule IDs, triggers, identifier, priority, and active status, in that order. A later storage failure can leave earlier changes applied; processing can observe intermediate configurations.
- Omitted fields are preserved; `{}` returns the existing config without changing `last_modified_at`. Updates do not rebuild keys/links or requeue entries, so changing identifiers or rules can leave historical keys based on the previous config semantics.

## Matching Keys and Direction

- `LinkSearchIdentifier::V1` supports `Single` and `Composite` mappings between `transaction_field` and `confirmation_field`. Fields are extracted from the decrypted `DomainEntry`, including full metadata rather than only `exposed_metadata`.
- Identifier extraction rejects missing fields and null, array, or object metadata values; empty strings are accepted, including individual composite components. Values are used without trimming.
- The extracted value is hashed with SHA-256 for equality lookup and encrypted with the merchant encryptor for storage in `search_entry_link_key.hashed_value` / `encrypted_value`. `target_key` holds the field name(s); `entry_link` stores the entry pair and link status, with no key/value columns.
- A stored key advertises the **opposite** side and its field name, while retaining the originating entry's rule ID. Lookup uses the arriving entry's own side and field name.
- For example, transaction field `metadata.payment_id` mapped to confirmation field `metadata.reference`: a transaction entry stores `target_side = confirmation`, `target_key = metadata.reference`. A confirmation searches that same pair with its extracted reference value.
- Lookup also requires the same `link_config_id`, hashed value, and compatible posted/non-posted key status. Account/profile scope comes through the selected config.

Example composite identifier stored in `link_search_identifier`:

```json
{
  "link_search_version": "v1",
  "matching_key_type": "composite",
  "keys": [
    { "transaction_field": "metadata.payment_id", "confirmation_field": "metadata.reference" },
    { "transaction_field": "currency", "confirmation_field": "currency" }
  ],
  "delimiter": "pipe"
}
```

- Composite pairs are sorted by `transaction_field.to_string()` for **both** sides. Independently sorting confirmation fields would misalign values when names differ.
- Shared `CompositeSearchIdentifierExt` lives in `crates/recon/src/domain/search_identifier.rs`; reconciliation retains its own ordering by `source_field.to_string()`.
- Delimiters are `pipe` (`|`) or `none` (concatenation). Values are not escaped or length-prefixed: embedded pipes or ambiguous concatenation can collide. Config authors must choose unambiguous inputs.
- Link-config create/update APIs reject empty composite keys, but not duplicates. Direct `LinkConfigInterface` insertion bypasses this API validation; recon-rule validators do not run here.

## Matching Outcomes and Persistence

`EntryStatus::is_posted()` is true only for `Posted`; `Matched`, `Pending`, `Mismatched`, `Archived`, and `Void` use the non-posted path. Expected placeholders stay out through `linking_status = not_applicable`, which explicit enqueue does not accept.

| Incoming entry | Eligible matching keys | Outcome |
|---|---|---|
| Non-posted, no matches | `Open` or `Matched` | Insert `Open` key; no link row |
| Non-posted, one or more matches | `Open` or `Matched` | Insert one `Provisional` link per match; new and matched keys become `Matched` |
| Posted, no open posted matches | `PostedOpen` | Insert `PostedOpen` key; no link row |
| Posted, exactly one open posted match | `PostedOpen` | Insert `Final` link; both keys become `PostedMatched` |
| Posted, multiple open posted matches | `PostedOpen` | Persist `MultiplePostedLinksFound` failure |

- Posted lookups initially include `PostedMatched`, but context creation filters them out. Under serialized processing, already-finalized keys are never reused for another final link.
- Posted and non-posted keys do not match each other. A later posted entry creates its own final-link path; it does not promote an existing `Provisional` row.
- `process_context` atomically updates matched keys, inserts links, inserts the incoming key, and marks the incoming entry `Processed`. Non-posted link inserts are chunked at 1,000 rows within the transaction.
- Finding candidates happens before that write transaction. Serialization of work is an execution contract, described below.

## Processor and Worker Ownership

- Server initialization spawns a separate linking processor, initially `Stopped`. Starting the recon processor does not start linking.
- Admin endpoints: `GET /linking_processor/status` returns `{"status":"stopped"}` or `{"status":"running"}`; `POST` to the same path accepts that shape and returns HTTP 200.
- Status-control authentication is `AdminApiAuth` with the `api-key` header. Status lives in an in-process watch channel, is not persisted, and controls that application instance. Enqueue uses JWT authentication instead (see below).
- Current processing fetches one pending entry at a time, ordered by `(created_at ASC, id ASC)`. It sleeps five seconds on an empty queue or a failed queue query.
- **Future concurrency must partition by account.** No two workers may process entries belonging to the same account concurrently; processing within an account remains serialized. Since a link config is scoped to one account, this keeps both sides of a match under one worker.
- Account ownership must remain exclusive across application instances and worker handoffs. This partitioning is planned, not implemented by the current global pending query. `BATCH_SIZE = 1` is only local serialization.
- There are no row claims or locks around candidate selection. Unique indexes prevent duplicate entry/config keys and duplicate config/entry pairs, but do not enforce one final counterpart per entry. Concurrent workers can still consume the same posted candidate for different incoming entries; preserve the account ownership contract.

## Inline Processing

- `POST /linking_processor/{entry_id}/process` accepts a physical entry ID with no body and uses `AdminApiAuth` (`api-key`), not a profile JWT. It has no JWT profile ownership check; a JWT alone cannot authorize it.
- Only `entry.linking_status = Pending` is eligible. Failed, Processed, and NotApplicable return 400 `LP_802`; an unknown entry returns 404 `LP_801`. To retry a Failed entry, fix the cause and use JWT enqueue before processing.
- The request runs the same core pipeline inline, even when the background processor is Stopped, and does not change processor status. HTTP 200 can contain JSON `"success"` or `{"failure":"<reason>"}`: recorded processing failures are successful HTTP responses. Success can mean a searchable key was created without a counterpart.
- Direct infrastructure/data-integrity errors still propagate as HTTP 500 and can leave the entry Pending; see [failure policy](#failure-policy). The entry response does not expose linking state, and an empty linked-entry list alone cannot distinguish pending, failed, or processed-without-match entries.
- **Inline calls must share the account serialization contract.** The Pending check is not a claim or lock, and inline calls can race each other or the background worker. A stopped status does not acknowledge completion of already-running work; ensure no same-account work remains in flight before processing manually.

## Reading Linked Entries

- `GET /entries/{entry_id}/linked_entries` uses JWT `ProfileReconTransactionRead`. It checks the requested entry's account against the JWT profile before querying; missing or foreign-profile entries return 404, while an owned entry with no links returns `[]`.
- Lookup works from either side using the physical entry ID. Each result contains `entry_link_id`, `link_config_id`, link `status` (`final` / `provisional`), and the **counterpart** as a full `GetEntryResponse`; its nested `entry.status` is the separate reconciliation status.
- Results include all matching link rows, ordered by `(entry_link.created_at ASC, entry_link.id ASC)`, without pagination, latest-version filtering, or final-only filtering. Preserve historical provisional counterparts rather than collapsing by logical transaction ID.
- The query filters `entry_link.profile_id` and joins the counterpart entry directly; it does not revalidate every counterpart account or require an active config. Correct link-row ownership remains a write-time invariant. Counterparts use the normal merchant-scoped entry decryption path; a conversion failure fails the whole response.

## Explicit Enqueue and Retry

- `POST /linking_processor/{entry_id}/enqueue` takes a physical `entry.id`, with no request body. It uses `JwtAuth::basic(Permission::ProfileReconTransactionWrite)` and returns HTTP 200 after resetting `linking_status` and `linking_data` to Pending. It does not run linking inline or start the processor.
- Ownership is checked through `find_account_by_profile_id_and_account_id` against the JWT profile before checking status. A missing entry or an account outside that profile returns 404 `LP_801`; storage failures map to 500 `LP_800`.
- **Only Failed linking status can be enqueued.** `EntryLinkingStatus::enqueuable_statuses()` defines the allowed set. Pending, Processed, and NotApplicable return HTTP 400 `LP_802` (`InvalidEntryLinkingStatus`), with the current and expected statuses in the error message. Eligibility uses `entry.linking_status`, independently of reconciliation `entry.status`.
- A successful reset replaces prior failure data, leaves reconciliation state and existing keys/links unchanged, and preserves `created_at`, so queue order is still the entry's original creation order. A request that reads the entry as Pending is rejected; enqueue is a failed-entry retry, not a way to re-link Processed entries.
- Enqueue checks status before a separate update by entry ID; it does not claim the entry or perform a conditional status transition. Retry requests for the same entry must be coordinated with processing.
- Retry Failed entries after resolving the cause through the [config APIs](#config-api-and-update-constraints), then enqueue. The pending entry can be handled by the background worker or the admin-only [inline endpoint](#inline-processing).

## Failure Policy

There are two deliberate error paths, following the recon engine's separation between recorded processing failures and blocking infrastructure/data-integrity failures.

| Failure path | Entry state and queue behavior |
|---|---|
| `process_linking_or_fail!` / `process_linking_error_context` | Log the error, persist `Failed` and structured `linking_data`, return `Failure(reason)`; the pending query moves on |
| Direct `LinkingProcessorError` propagation | Leave the entry pending; a persistent error on the oldest entry blocks queue progress and requires investigation |

- Recorded failures cover entry decryption, config selection (`NoLinkConfigFound` included), missing identifier fields, ambiguous posted matches, and failures during context creation/persistence.
- `LinkingProcessingError::DatabaseError` is stored as `LinkingProcessingErrorTypeV1::InternalError`, not a separate database-error payload. If persisting the failure itself fails, that error propagates and the entry can remain pending.
- **No transformation ID** is explicitly recorded as `TransformationConfigMissing`. **An ID whose lookup fails** follows the blocking path instead.
- Transaction lookup/decryption and transformation/schema lookup failures propagate directly. These relationships are expected to be valid from transaction creation; do not silently skip the entry or convert this policy to best-effort processing.
- The current loop discards per-entry results and immediately retries pending entries; it does not change processor status to `Stopped` on a hard failure. A reported `running` status therefore does not guarantee queue progress.
- Failed entries are not automatically retried; explicit enqueue resets their linking state to Pending while preserving existing keys and links.

## Storage and Migration Gotchas

- `entry.linking_status` and versioned JSONB `linking_data` are separate from reconciliation `status`/`data`. `EntryUpdate::LinkingUpdate` derives the linking status from the data and leaves reconciliation state untouched.
- Pending data serializes as `{"data_version":"v1","status":"pending"}`. Failures additionally carry `failure_data_version`, an `error_type` object, and its `error_type_version` / `error_type` tags.
- `EntryNew::new` sets Expected entries to `NotApplicable` and other statuses to `Pending`; domain direct/expected constructors initialize the corresponding states too. `DiscardUpdate` deliberately leaves them unchanged.
- Migrations `2026-10-09-120000` through `120300` create `link_config`, `entry_link`, `search_entry_link_key`, and add the entry linking fields. The backfill marks rows currently `Expected` as `not_applicable` and every other status, including archived rows, as `failed` with a `no_link_config_found` reason. Historical rows are not automatically queued; new-entry constructors still default non-Expected entries to Pending.
- The unique index on `link_config(profile_id, account_id, priority)` covers active and inactive configs and supports profile/account lookup. There is no separate active-only config index.
- Unique indexes enforce `search_entry_link_key(entry_id, link_config_id)` and `entry_link(link_config_id, confirmation_entry_id, transaction_entry_id)`. Neither includes status, and inserts have no upsert/conflict-ignore behavior. The linking tables have no foreign keys; profile/account/rule consistency remains application-owned.
- Other indexes support key lookup by config/target key/hash, links by either entry ID, and pending-entry ordering.
- No migration converts legacy `link_with_rule` JSON into `link_config`. Supply link configs before processing entries; starting the processor alone does not enqueue historical rows.

Runtime failure JSON for `NoLinkConfigFound`:

```json
{
  "data_version": "v1",
  "status": "failed",
  "failure_data_version": "v1",
  "error_type": {
    "error_type_version": "v1",
    "error_type": "no_link_config_found"
  }
}
```

## Source Map

Paths are relative to the repository root.

- `crates/recon/src/domain/linking_processor.rs`: processing pipeline, error handling, context types, `LinkingProcessor` trait.
- `crates/recon/src/domain/linking_processor/link_orchestration.rs`: active config discovery and priority selection.
- `crates/recon/src/domain/linking_processor/implementations.rs`: matching outcomes and atomic persistence; the adjacent `implementations/` directory contains `link_trigger.rs` and `link_matching_key.rs` for evaluation/extraction.
- `crates/recon/src/core/linking_processor.rs` and `crates/recon/src/routes/linking_processor.rs`: background loop, admin status/inline processing controls, and JWT-scoped enqueue.
- `crates/recon/src/core/entry.rs`, `crates/recon/src/routes/entry.rs`, and `crates/recon/src/db/entry_link.rs`: profile-scoped linked-entry lookup and counterpart responses.
- `crates/recon/src/core/link_config.rs` and `crates/recon/src/utils/link_config.rs`: config API persistence, request validation, and API/storage conversion.
- `crates/recon/src/utils/linking_processor.rs` and `crates/recon/src/errors/linking_processor.rs`: persisted failure conversion, failure macro, and API errors.
- `crates/common_enums/src/linking.rs`, `crates/common_types/src/entry_linking.rs`, and `crates/common_types/src/link_config/`: statuses and versioned payloads.
- `crates/diesel_models/src/query/`: `entry.rs` selects pending entries; `link_config.rs`, `search_entry_link_key.rs`, and `entry_link.rs` provide config lookup and link persistence.
- `crates/recon/tests/api/linking_processor.rs` and `link_config_test.rs`: API coverage for arrival order, provisional fan-out, posted candidate consumption, config isolation/selection, account-scoped priority uniqueness (including inactive configs and self-updates), empty-trigger rejection on both create and update, failure/retry, Expected-entry rejection, and config update constraints.

Related domains: [transactions](../transactions/AGENT-NOTES.md), [recon rules](../recon-rules/AGENT-NOTES.md), [reconciliation](../reconciliation/INDEX.md).
