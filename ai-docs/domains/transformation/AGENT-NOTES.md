# Transformation - Agent Notes

## Overview
Transformation bridges ingestion and reconciliation by converting external file data into staging entries. One ingestion can trigger multiple transformations in parallel, each producing staging entries for different accounts.

## Critical Rules

### Authentication Pattern
<!-- AI-UPDATED: 2025-11-14 - V1 transformation configs removed -->
- **Execute operations**: NoAuth - system-level automation (changed from AdminApiAuth)
- **V2 CRUD operations**: JWT Bearer for both creation and reads (merchant_id/profile_id from auth context)
- **WHY**: V2 uses JWT for tenant isolation and user context
- **CRITICAL**: V2 endpoint `/transformations/v2/configs` requires Bearer token, not API key
- **DEPRECATED**: V1 transformation configs and AdminApiAuth create endpoint completely removed

### Parallel Processing Model  
- One `ingestion_id` → multiple active `transformation_configs` → multiple `transformation_histories`
- All configs for an ingestion execute **in parallel** using futures::try_join_all
- If file retrieval fails, **ALL** configs for that ingestion are marked failed
- If individual config processing fails, only that specific config fails

### Status Lifecycle (One-Way Only)
```
Pending → Processing → Processed | Failed
```
- **Pending**: Created before file processing starts
- **Processing**: Set when individual transformation begins  
- **Processed**: Success with TransformationSuccessData (staging entry IDs + metrics)
- **Failed**: Failure with TransformationFailureData + failure stage
- **Critical**: No status rollbacks - transitions are one-way only

### TransformationHistoryUpdate Pattern
<!-- AI-UPDATED: 2025-07-21 - Type-safe update enum -->
- Uses enum variants instead of separate parameters for status updates
- **Processing**: Simple status change, no data
- **Processed { processed_data }**: Success with transformation results
- **Failed { failure_data }**: Failure with error details
- Automatically sets `processed_at` for terminal states (Processed/Failed)
- Status is implicit in variant name - prevents invalid state combinations

## Common Gotchas

<!-- AI-UPDATED: 2025-10-29 - V2 transformation config validation errors -->
### V2 Configuration Validation
- **MetadataSchemaNotFound**: New specific error (TR-1008) when metadata_schema_id doesn't exist
- **Validation Pattern**: V2 uses `to_not_found_response()` for account and schema validation
- **Auth Context Dependency**: V2 requires merchant_id and profile_id from JWT, not request body
- **Critical**: V2 config creation will fail if metadata schema is not found in the requesting profile

<!-- AI-UPDATED: 2025-08-13 - Transformation config loading pattern for duplicate detection -->
### Transformation Config Loading Pattern
- Missing transformation configs are expected and should not fail duplicate detection
- Use early return pattern: `let Ok(config) = find_config().await else { return Ok(()); }`
- Alternative: `match find_config().await { Ok(config) => config, Err(_) => return Ok(()) }`
- **WHY**: Not all staging entries have associated transformation configs (direct API uploads)

### Error Handling Strategy
<!-- AI-UPDATED: 2026-08-18 - per-row outcomes replace error strings; FileParsing stage added -->
- File processing errors are **captured, not thrown** - transformation continues
- A bad row becomes a `TransformationRowResult` with a `Failed` outcome carrying located diagnostics; it never stops the other rows
- Five failure stages, in pipeline order: FileRetrieval, **FileParsing**, TransformationExecution, **TransformationReportUpload**, StagingEntryInsertion. `FileParsing` means "your file is malformed" (the parser rejected the whole file), `TransformationExecution` means "our engine broke" — `core/transformation.rs` matches on `TransformationError::InvalidFileFormat` to pick between them. The enum is only ever constructed, never matched on, so adding a stage is safe.
- `update_transformation_history_failed` calls are fire-and-forget (`let _ =`)

### Configuration Versioning
<!-- AI-UPDATED: 2025-11-14 - V1 transformation configs completely removed -->
- **DEPRECATED**: V1 transformation configs completely removed from system
- Uses tagged enum: `TransformationConfigData::V2(TransformationConfigDataV2)` only
- V2: Schema-driven transformation with `metadata_schema_id` and `parsing_config`
- Config stored as JSONB with embedded account_id and profile_id for denormalization
- **BREAKING CHANGE**: All legacy V1 transformation configs removed

<!-- AI-UPDATED: 2025-11-14 - V1 unique field implementation removed -->
- **V2 Unique Field Implementation**: Only V2 configs supported for unique field extraction
  - `TransformationConfigData::unique_field(metadata_schema: Option<&MetadataSchemaData>) -> String`
  - **DEPRECATED**: V1 configs and their unique_field methods completely removed
  - V2 configs: Extracts unique field name from metadata schema's unique constraint
  - Fallback: Returns "metadata_schema_driven" if metadata schema not provided for V2
  - **Simplified**: Only V2 enum variant remains in TransformationConfigData

### File Processing Constraints
<!-- AI-UPDATED: 2025-10-28 - Added order_id validation constraint -->
<!-- AI-UPDATED: 2026-08-18 - encoding failures are per-row, except the CSV header -->
- **Encoding failures are per-row, with one exception.** A non-UTF-8 *data* row becomes a `RowParsingOutcome::Rejected` at its real line number and the rest of the file still processes. The exception is the **CSV header**: `reader.headers()` failing rejects the whole file as `ParsingFailure::InvalidFormat`, because there are no column names left to map anything onto.
- **A failing main field rejects its whole row, and reports only that one error.** `validate_row` short-circuits on currency, amount, balance_direction, effective_at and order_id in that order, so no partial entry is ever produced. **Metadata-field failures accumulate instead** — one row reports every bad metadata field at once. The asymmetry is deliberate; see `data_validator.rs`.
- Row rejection never stops processing other rows
- File paths provided by ingestion system are trusted input (no additional validation)
- **order_id Field**: Exceeding 64 characters causes validation error at metadata schema level
  - MaxLength(64) validation prevents database errors since column is VARCHAR(64)
  - Validation happens during staging entry creation, not at transformation time
  - Error message will indicate "ValueTooLong" with max_length: "64"

<!-- AI-UPDATED: 2025-07-24 - PathBuf type safety for file operations -->
- File paths now use `PathBuf` type instead of `String` for type safety
  - `TransformationTriggerRequest.file_path` is now `PathBuf`
  - Must convert to `&str` for file storage retrieval: `.to_str().ok_or(InvalidFileName)?`
  - Returns `TransformationError::InvalidFileName` if path contains non-UTF8
  - API still accepts String in JSON, converted to PathBuf internally

### Database Relationships (Logical Only)
- `transformation_config.ingestion_id` references external system (no FK constraint)
- `transformation_history.transformation_id` references config.id (FK-like but not enforced)
- Profile/account IDs denormalized for query performance over referential integrity
- **No cascading deletes** - manual cleanup required for data integrity

## Design Decisions

### Why Administrative Authentication for Execute?
Transformation execution is system automation triggered by ingestion pipeline, not user-initiated operations. CRUD operations use JWT for profile isolation.

### Why Parallel Config Processing?
Single ingestion can feed multiple accounts/transformations simultaneously. Each config processes the same file data for different business contexts.

### Why Error Collection vs Failure?
Real-world business files often contain invalid lines. System expects and handles partial success scenarios gracefully.

### Why Denormalized Relationships?
Query performance prioritized over strict referential integrity. Transformation queries are frequent and performance-critical.

### Why Tagged Enum Versioning?
Future transformation types can be added without breaking existing stored configurations. V2, V3, etc. can coexist.

## V2 Configuration Implementation

<!-- AI-UPDATED: 2025-10-29 - V2 transformation config foundation -->
### V2 vs V1 Architecture Differences
- **Route Path**: V2 uses `/transformations/v2/configs` instead of `/transformations/configs`
- **Authentication**: V2 uses JWT Bearer (tenant-scoped) vs V1 AdminApiAuth (admin-only)
- **Request Structure**: V2 excludes merchant_id/profile_id from request body (obtained from JWT)
- **Schema Integration**: V2 includes metadata_schema_id field for future schema-driven transformation
- **Error Handling**: V2 introduces MetadataSchemaNotFound (TR-1008) specific error type

### V2 Implementation Status (Complete)
<!-- AI-UPDATED: 2025-11-03 - V2 schema-driven transformation implementation completed -->
- **API Models**: `CreateTransformationConfigV2Request` with schema integration ✅
- **Common Types**: `TransformationConfigV2` struct with parsing config ✅
- **Database**: V2 configs stored using existing versioned enum pattern ✅
- **Routes**: `/transformations/v2/configs` endpoint with JWT authentication ✅
- **Core Logic**: Account and schema validation with proper error handling ✅
- **Transformation Execution**: Full schema-driven transformation pipeline implemented ✅
- **Schema-Driven Processing**: Complete metadata schema integration with parsing → mapping → validation → encryption ✅

<!-- AI-UPDATED: 2026-05-14 - In-place update endpoint added -->
<!-- AI-UPDATED: 2026-09-07 - is_active became updatable; write ordering and pause semantics documented -->
### In-Place Update (`PUT /transformations/configs/{transformation_id}`)
- **Updatable fields**: `name` (String), `skip_configs` (Vec\<SkipConfig\>), `parsing_config` (ParsingConfig), `metadata_schema_data` (MetadataSchemaData), `is_active` (bool) — all optional
- **`skip_configs` / `parsing_config` patch semantics**: handler reads the existing `TransformationConfigData::V2` from the DB, overlays only the provided fields, writes the merged result back as a single `ConfigUpdate` — the other config fields are preserved
- **`metadata_schema_data` update**: updates the schema already linked to the config **in-place** via `update_metadata_schema_by_id`; the `transformation_config.metadata_schema_id` pointer is **not changed**; uses `MetadataSchemaUpdate::SchemaUpdate` → `MetadataSchemaUpdateInternal` (`AsChangeset`) which always stamps `last_modified_at`
- **`TransformationConfigUpdate`** has three variants: `NameUpdate { name }`, `ConfigUpdate { config }` and `ActiveStatusUpdate { is_active }`; all map to `TransformationConfigUpdateInternal` (Diesel `AsChangeset`). There is no `treat_none_as_null`, so a `None` field is dropped from the `SET` clause — this is what keeps one variant from clobbering another's column
- **Up to three separate DB writes, none transactional**: each provided field group fires its own write and stamps its own `last_modified_at`, so a partial failure leaves a partial update
- **`ActiveStatusUpdate` is deliberately the last write**: ordered after the `metadata_schema_data` update. Reversed, a single PUT carrying both would activate the config while the old schema is still live, and a failing schema write would strand it permanently active on that stale schema

### V2 Schema-Driven Transformation Pipeline (Implemented)
<!-- AI-UPDATED: 2026-08-18 - Rewritten for the row-oriented engine; collection-level APIs removed -->
The pipeline is **per-row**, not per-collection. `DefaultTransformationEngine::transform_input` (in `domain/transformation.rs`) walks rows one at a time through parse → skip → map → validate; the whole-file steps sit outside it in the `transform_file` application adapter.

1. **Fetch Schema** (`transform_file`): metadata schema by `metadata_schema_id` + `profile_id`
2. **Parse File**: `parsing_config.parse(&file_data)` → `FileParsingOutcome::{Parsed { rows }, Rejected { failure }}`. A `Rejected` becomes `TransformationError::InvalidFileFormat` → `TransformationFailureStage::FileParsing`
3. **Per row** inside `transform_input`: `evaluate_skip` → `schema.map_row(row)` → `schema.validate_row(&mapped)`, producing one `RowOutcome::{Success, Skipped, Failed}` per input record
4. **Stage in place**: each row maps 1:1 onto a `StagedRow` — skips and failures are already `Completed { result }`, successes become `AwaitingEncryption { entry_data }`
5. **Batch Encrypt**: `encrypt_row_metadata` does the I/O only — it collects the `AwaitingEncryption` UUIDs into one batch and returns `FxHashMap<String, Option<Encryption>>`
6. **Resolve**: `staged_rows.into_iter().map(resolve_staged_row).collect()` produces the `TransformationExecutionResult`. `resolve_staged_row` is pure and 1:1, returning `(TransformationRowResult, Option<StagingEntryNew>)` — it builds the entry via `build_staging_entry` on a hit, and emits a `RowFailureStage::Encryption` rejection on a miss
7. **Report, then Insert, then Record** (`core/transformation.rs`): generate and upload both reports **first**; only if that succeeds are the staging entries batch-inserted and the history written as `Processed` with the report keys plus outcome counts in `TransformationData::V2`

- **No collection-level trait methods exist.** `map_entries`, `validate_entries`, and `skip_rows_based_on_config` were removed; the singular `RowMapper::map_row`, `RowValidator::validate_row`, and `evaluate_skip` replace them. Traversal belongs to the engine, so the single-row API paths (`core/staging_entry.rs`, `core/exception_management.rs`) reuse the same validators via `ParsedRow::from_api_fields`.
- **Nothing in the pipeline returns `Result` for bad data.** Parsing, skipping, mapping and validation all answer with outcome enums (`FileParsingOutcome`, `RowParsingOutcome`, `SkipDecision`, `RowValidationOutcome`, `FieldValidationOutcome`). `Result` is reserved for the engine itself failing. **Why:** a bad row is an answer about the input, like a 4xx, not a 5xx — and short-circuiting on the first bad row is what lost every other row's errors.

### V2 Implementation Patterns
<!-- AI-UPDATED: 2026-08-18 - Per-row located diagnostics replace flat error strings -->
- **Errors carry locations, not formatted strings**: `TransformationDiagnostic { location: SourceLocation, error: String }`. The old `"Line {}: {}"` formatting is gone — the report writer in `utils/transformation.rs` is the only place that renders a location into prose.
- **`SourceRecord` identifies a whole input record; `SourceLocation` identifies the most precise spot** (`FileRow` / `FileCell` / `FileRange` / `ApiRecord` / `Field`). Both live in `domain/transformation/types.rs` and serialize straight into the JSON report; do not add duplicate storage-layer twins.
- **Column numbers are 1-indexed everywhere and `FileRange` includes both ends.** Fixed-width configs use a 0-based `index`, so its parser emits `start_column: index + 1, end_column: index + width`.
- **Field errors get their location from the value they are about**, carried on `LocatedValue<T>` from parsing through mapping. Never re-derive a location by looking the identifier back up in the schema.
- **There is no UUID → line-number side table.** `TransformationStagingEntryData.uuid` exists *only* as an encryption-batch lookup key. Position is carried by the row's own `source`, and order is carried by `Vec` order — see the ordering rule below.
- **Batch Processing**: Follow exact V1 encryption patterns using `FxHashMap<String, Option<serde_json::Value>>`
- **Database Access**: V2 requires `state: &AppState` parameter for metadata schema database lookups
- **Processing Mode**: All V2 transformations use `StagingEntryProcessingMode::Transaction` (hardcoded)

<!-- AI-UPDATED: 2026-08-18 - report rows must stay in file order -->
- **The row vector is never divided by outcome — file order is the report's contract.** `transform_file` maps engine rows 1:1 onto `Vec<StagedRow>`, then maps that same vector through `resolve_staged_row` and collects it. The encryption batch is built by a read-only `filter_map` **view** over the vector, never by extracting the successes into a second collection. Accumulation lives in `impl FromIterator<(TransformationRowResult, Option<StagingEntryNew>)> for TransformationExecutionResult`, so no caller ever holds two parallel vectors. **Why:** the code previously partitioned into (skipped ∪ failed) vs (validated) and appended the encrypted results onto the tail, so `report-v1.csv` listed every error first and every success last — an operator could not line it up against their source file. **How to apply:** do not reintroduce a two-accumulator fold here, and do not "fix" order with a trailing sort — it is O(n log n) *and* has no sound key, since `SourceRecord` derives no `Ord` and `ApiRecord { index }` is `None` at both API call sites. `staging_entries_new` lands in the DB in file order for the same reason.

### Transformation History Data (`TransformationData`)
<!-- AI-UPDATED: 2026-08-18 - V2 per-row report artifacts added -->
- **V1 is frozen — read-only.** Existing JSONB rows still deserialize through it, but every new write is `V2`, including failures (`update_transformation_history_failed`).
- **`TransformationDataV2::Processed` stores counts and report file keys, not row arrays.** `success_count + ignored_count + failed_count` partitions all input records. **Why:** row arrays in JSONB made the history row unbounded; the artifacts live in file storage instead, under `transformations/{transformation_id}/{history_id}/report-v1.{csv,json}`.
<!-- AI-UPDATED: 2026-08-19 - reports are now a precondition for insertion, not best-effort -->
- **Reports are a precondition for staging insertion, not a best-effort afterthought.** Both uploads run *before* `batch_insert_staging_entries`; if either the JSON or the CSV fails to serialize or upload, the transformation stops at `TransformationFailureStage::TransformationReportUpload` and **no staging entries are written**. **Why:** staged data a merchant cannot get a report for is worse than no staged data — the failure is retryable from a clean slate, whereas a partial success leaves entries in the DB that nothing explains. **How to apply:** the two uploads are all-or-nothing; don't "degrade" to uploading only one.
- **Two error variants, one failure stage.** `ReportGenerationFailed` (TR-1015) covers serialization — `serde_json::to_vec_pretty` and the `csv::Writer` — and `ReportUploadFailed` (TR-1016) covers key construction and `file_storage.upload_file`. Both record the single stage `TransformationReportUpload`; the error text is what distinguishes them, so a new report failure mode needs a new *error variant*, not a new stage.
- **The stored failure message is built with `format!("{e:#}")`, and that is load-bearing.** In error-stack 0.4.1 `Display for Report` **discards attachments** and, in non-alternate `{e}`, prints only the outermost context before breaking. So `.attach_printable(...)` is visible in logs (`Debug`) but **never** in the message persisted to `transformation_history`. **How to apply:** anything an operator must read out of the history row belongs in the error variant's `#[error(\"...\")]` text, not an attachment; and don't drop the `:#` — plain `{e}` would silently discard the underlying serde/storage cause.
- **The storage type is tight, the API type is not — and that asymmetry is deliberate.** `TransformationDataV2::Processed` holds `csv_report_file_key: PathBuf` and `json_report_file_key: PathBuf`, **not** `Option`, because reports are now a precondition: a row can only reach `Processed` if both uploaded. The API type `TransformationResultData::Processed` keeps `Option<PathBuf>` because it is the **union of V1 and V2** — V1 histories predate reports entirely and map to `None`. **How to apply:** don't "simplify" the API type to match the storage type; it would break every V1 history. `ReportNotAvailable` (TR-1014) is still reachable — for `data: None` (Pending/Processing), for V1 rows, and for `V2::Failed`.
- **CSV is operator-facing, JSON is machine-facing.** CSV is one row per input record with `source_type,identifier,outcome,details` (`ReadableTransformationReportRow`); JSON pretty-prints `Vec<TransformationRowResult>` so a large report is not one unopenable line. **Both are serialized from the same row vector** — do not allocate a second detailed-report model or split it into transformed/skipped/rejected arrays.
- **`total_count` changed meaning from V1**: it is now data records only (`success + failed + ignored`). V1 counted raw file lines *including the header*, so the two are not comparable, and neither matches the row count a spreadsheet shows.
- **Skipped-row reports store the matched conditions AND the row's value for each** (`MatchedCondition::ValueSkip { config, field_value }`). **Why:** a condition alone ("skip when `profile_id != abc`") does not tell an operator why *this* row matched, and an index into `skip_configs` would go stale because the config can be edited in place. `field_value` is the **raw** pre-transformation value.
- **Download API:** `GET /transformations/history/{transformation_history_id}/reports/{report_format}` accepts `csv` or `json` and requires `ProfileReconTransformationRead`. A profile mismatch returns `TransformationHistoryNotFound` (TR-1013, 404) rather than a 403 — **deliberate**, so the endpoint cannot be used to probe which history IDs exist in other profiles. A missing report key returns `ReportNotAvailable` (TR-1014, 404).

## Skip Configurations
<!-- AI-UPDATED: 2026-04-27 - skip_configs added to TransformationConfigV2 -->

- **AND/OR semantics are not visible in the type names**: conditions inside one `SkipConfig` are AND-ed; multiple `SkipConfig`s in the `Vec<SkipConfig>` are OR-ed. A row is skipped iff at least one `SkipConfig` has all its conditions match.
- **Empty `conditions` vec is vacuously true** — under AND semantics it matches every row, silently dropping the entire file. `validate_transformation_config_request` rejects this on create with `TR/1011 EmptySkipConfigConditionsFound`. **If a new entry point ever accepts `skip_configs`, it must call this validator or inline the same check** — the guard is not enforced at the type level.
- **API type is unversioned, internal type is versioned**: `api_models::transformation::SkipConfig` is a plain struct; `common_types::transformation::SkipConfig` is `enum { V1(SkipConfigV1) }`. `ToDiesel` always wraps the API form as `V1`. When adding V2, plan the API-side discriminator before changing the wire format.
- **`ValueSkip` against a missing column is a silent no-op**: if `identifier` is not present in the parsed row's `fields`, `match_row` returns `None` — the row is kept, no error is raised. A typo'd identifier won't fail loudly; it will simply never match. Pinned by `value_skip_missing_identifier_does_not_skip`. **Why:** a misconfigured identifier should not abort an entire file when only some rows might have been intended to skip — operators prefer permissive evaluation here over fail-fast.
- **`ValueSkip.value` is `Option<String>`, compared one-to-one with the field**: `None` (cell missing/blank) and `Some("")` (cell present with empty string) are **distinct**. To match a blank cell, configure `value: null`; to match a literal empty string, configure `value: ""`. Pinned by `value_skip_none_value_matches_only_none_fields` and `value_skip_empty_string_does_not_match_none_field`.
- **`ValueSkip.transformation_rules` transforms the raw field before comparison**: the field supports the same ordered string transformations as metadata string fields and defaults to an empty vec for backward compatibility. This is required for fixed-width CSV values such as merchant IDs padded with spaces; configure `trim` before `equals`/`not_equals`. If a transformation fails, the condition evaluates to false so a malformed rule cannot accidentally skip rows.
- **`#[serde(default)]` on `skip_configs`** keeps older clients working — omitting the field deserializes to an empty vec, no skip step runs.
<!-- AI-UPDATED: 2026-08-18 - line_uuids removed; the row carries its own source -->
- **Skip reads the row's own `source`, not a side table**: `RowSkip::LineNo { line_number }` matches `ParsedRow.source` when it is `SourceRecord::FileRow` — the original *file* line number, already accounting for header rows (1-indexed for CSV, Excel-style for XLSX). Don't reason about it as a Vec index. An `ApiRecord` row never matches a `LineNo` condition.
- **Evaluation returns its evidence, not a bool**: `ConfigSkipEvaluator::match_row` and `ConditionSkipEvaluator::match_row` return `Option<..MatchedCondition..>`, capturing the field value at the moment the operator matched. **Why:** the reported value and the evaluated value cannot drift, and the row is read once. Don't add a second pass that re-reads the field just to build the reason.
<!-- AI-UPDATED: 2026-04-27 - Contains/NotContains operator gotchas -->
<!-- AI-UPDATED: 2026-07-27 - generalised to the full substring family; anchored prefix/suffix operators added -->
- **The substring family treats any `None` side as "do not skip"**: `Contains`, `NotContains`, `StartsWith`, `NotStartsWith`, `EndsWith` and `NotEndsWith` all return `false` (row kept) if either the row's field value or the configured `value` is `None`. This is **asymmetric with `NotEquals`**, which skips when `field=Some(_)` vs `value=None`. **Why:** substring semantics only make sense between two real strings; falling back to "don't skip" avoids dropping rows on missing data. **How to apply:** if you need "skip when the field is missing", use `Equals` with `value: null`, never a negated substring operator. Pinned by `value_skip_not_contains_none_field_keeps_row`, `value_skip_not_starts_with_none_field_keeps_row` and `value_skip_not_ends_with_none_field_keeps_row`.
- **`Contains ""`, `StartsWith ""` and `EndsWith ""` match every `Some(_)` field**: Rust's `str::contains("")`, `starts_with("")` and `ends_with("")` are all unconditionally `true`, so a misconfigured empty `value` will skip every populated row in the targeted column (rows where the field is `None` are still kept due to the rule above). **How to apply:** treat an empty `value` string on any substring-family operator as a config error during review — it's almost never what was meant. Pinned by `value_skip_contains_empty_string_matches_every_some_field` and its `starts_with`/`ends_with` siblings.
- **`StartsWith`/`EndsWith` are anchored, `Contains` is not**: `ACTIVE` matches `Contains "TIV"` but neither `StartsWith "TIV"` nor `EndsWith "TIV"`. **Why:** callers reaching for a prefix rule (batch-id prefixes, settlement reference codes) were previously forced to approximate it with `Contains`, which over-matches. Pinned by `value_skip_starts_with_is_anchored_not_substring` and `value_skip_ends_with_is_anchored_not_substring`.
- **`matches_when_both_present` in `skip_config.rs` is the single home of the `None` rule**: every substring-family arm delegates to it, so a new operator inherits the semantics for free. **How to apply:** negated operators must negate *inside* the predicate (`|f, e| !f.starts_with(e)`). Writing `!matches_when_both_present(..., |f, e| f.starts_with(e))` inverts the `None` handling and starts skipping rows on missing data — the whole reason the helper exists.
<!-- AI-UPDATED: 2026-08-18 - skipped rows are reported individually; ignored_lines counter gone -->
- **Skipped rows appear individually in both report artifacts**, and `ignored_count` is derived by counting `RowResultOutcome::Skipped` over the same row vector. The old `TransformationExecutionResult.ignored_lines` counter and the parser's always-zero `ParsingResult.ignored_lines` are both gone. **Why:** a bare count told an operator that rows were dropped but not *which* ones or *why* — the per-row `SkipReason` does both, and deriving the count from the rows keeps the number and the report from ever disagreeing.

## Metadata Schema Integration Details

<!-- AI-UPDATED: 2025-10-30 - Metadata schema unique constraint structure simplified -->
### Unique Constraint Structure Changes
- **Breaking Change**: `UniqueConstraintsV1` structure simplified from array to single constraint
  - **Before**: `{ items: Vec<UniqueConstraint> }` - supported multiple constraints
  - **After**: `{ constraint: UniqueConstraint }` - single mandatory constraint
  - **API Impact**: `unique_constraints: Vec<UniqueConstraint>` → `unique_constraint: UniqueConstraint`
  - **Database**: `common_types::metadata_schema` structure updated across all layers

### Unique Field Extraction Chain
<!-- AI-UPDATED: 2026-08-11 - Composite unique constraints: extraction chain and field-key construction -->
- `UniqueConstraintProvider::get_unique_constraint()` (`common_types::metadata_schema`) cascades `MetadataSchemaData → ReconMetadataSchema → ReconMetadataSchemaV1` down to `&UniqueConstraint`. `UniqueConstraintExt` (`get_unique_field_key`, `get_unique_field_delimiter`, `extract_unique_value`) cascades from there, `UniqueConstraint → UniqueConstraintV1 → UniqueConstraintType → UniqueConstraintTypeV1`, where the real per-version logic lives (not `get_field_name()` — stale name, doesn't exist)
- `UniqueConstraintProvider` trait + impls live in `common_types::metadata_schema` (no orphan-rule issue — both the trait and `MetadataSchemaData`/`ReconMetadataSchema*` are local there). `UniqueConstraintExt` trait + impls live in `recon`'s `domain::metadata_schema::implementations::unique_constraint`, since `UniqueConstraint` and friends are foreign types there
- `SingleField` or `CompositeField(CompositeUniqueField)` — the composite payload is a named struct; sorting and delimiter-joining live in `common_types::composite_field`, shared with `SearchIdentifier`/`GroupingField`
- **`extract_unique_value()` returns raw `Vec<(EntryField, Option<String>)>`, never `ProcessingResult`**: transformation must stay usable outside recon-engine context, so a missing field just comes back as `None` — deciding that's an error is the caller's job (`extract_unique_field` in `recon`'s `domain::recon_engine::utils`)
- `extract_unique_field(values, separator)` builds both the `FieldPathKey` and the joined value from the same `values` list, reducing each entry's `EntryField.to_string()` with `build_composite` rather than taking a separately-computed key. This only matches `get_unique_field_key()`'s output because `extract_unique_value()` already yields fields in the same (sorted, for composites) order the key is built from — if that ordering ever diverges between the two, the field name and the stored value silently stop corresponding to the same key
- `get_unique_field_key()` returns a `FieldPathKey` (`common_types::field_path_key` — top-level, not nested under `recon_rule`/`search_identifier`, since `search_expectations.target_key` needs the same type) stored as `search_duplicates.field_path`; `get_unique_field_delimiter()` returns `CompositeDelimiter::None` for `SingleField` (nothing to join) or the configured delimiter for `CompositeField`

### Duplicate Processing Integration
- **Process**: `get_metadata_schema_for_config()` retrieves schema for V2 configs
- **Extraction**: `transformation_config.config.unique_field(metadata_schema.as_ref())`
- **Usage**: Field name used for duplicate detection in reconciliation engine
- **Fallback**: Returns "metadata_schema_driven" if schema not available
- **Performance**: Single database call to fetch schema, then in-memory field extraction

## When Working Here
<!-- AI-UPDATED: 2026-04-21 - XLSX parsing engine added alongside CSV -->
### Parsing Engine
- **Versioned Config Pattern**: Three-level nesting: `ParsingConfig::V1(ParsingConfigV1::Xlsx(XlsxParsingConfig::V1(XlsxParsingConfigV1 {...})))` — follow this exactly when adding new formats
<!-- AI-UPDATED: 2026-08-18 - ParsingResult replaced by per-row outcomes -->
- **Parser output shape**: `FileParser::parse(&[u8]) -> FileParsingOutcome`, which is `Parsed { rows: Vec<RowParsingOutcome> }` or `Rejected { failure: ParsingFailure }`. Each row is `RowParsingOutcome::{Parsed(ParsedRow), Rejected(ParsingDiagnostic)}`.
  - **`ParsedRow` carries its own `source: SourceRecord`** and each field is a `LocatedValue<Option<String>>` under `fields` (not `raw_fields`). There is **no `line_uuids` side table and no UUIDs** — the old parallel `Vec<(usize, Uuid)>` and its end-of-pipeline rejoin are gone. File order is simply `Vec` order.
  - There is **no `total_lines_in_file` and no `ignored_lines`**. Header and preamble rows are never emitted, so `rows.len()` is data records only and will not match the row count a spreadsheet shows.
  - **A malformed line stays a row**: it becomes `RowParsingOutcome::Rejected` at its real line number rather than disappearing into a separate error list.
- **`ParsingFailure` has exactly three shapes**: `InvalidFormat { details }`, `HeaderNotFound`, `SheetNotFound`. All three reject the whole file and surface as `TransformationFailureStage::FileParsing`.
- **Row accumulation uses `map`/`collect`**: each parser maps one input line to exactly one `RowParsingOutcome` — no multi-accumulator `fold`, no `for` loops with `push`. Keeping the 1:1 shape is what makes file order free downstream.
- **An empty cell must parse to `None`, never `Some("")`**: all three parsers filter empty strings explicitly (CSV via `.filter(|v| !v.is_empty())`, XLSX via `Data::Empty` **and** `Data::String("")`, fixed-width via `is_empty()`). Dropping the filter silently inverts the documented `None` vs `Some("")` skip-config distinction and turns `RequiredFieldMissing` into a type error. Pinned by `csv_empty_cell_is_none_not_empty_string`.
- **CSV line numbers come from the reader position, corrected for pending CRLF bytes — not arithmetic**: `csv_record_line_number` starts from `record.position()` and adds any `\n` the `csv` crate left pending after terminating a record on `\r`; it falls back to `idx + 2` only when position data is unavailable. **Why:** raw `position.line()` labels the first CRLF data record as line 1, while plain arithmetic drifts after multi-line quoted fields. Pinned by `csv_crlf_line_numbers_start_after_header` and `csv_crlf_multiline_records_use_physical_start_lines`.
- **Zero-Copy CSV Reading**: `FileParser::parse` takes `&[u8]`, so no file bytes are copied.
- **XLSX Empty Header Cells**: <!-- AI-UPDATED: 2026-04-23 - Skip instead of error, match CSV behavior --> Columns whose header cell is empty are silently skipped. Data under those columns is dropped. Only columns with a non-empty header name appear in each row's `fields`.
<!-- AI-UPDATED: 2026-08-18 - XLSX coordinates are absolute, header_row_index is not -->
- **XLSX mixes two coordinate systems, and this is the easiest thing to get wrong here.** `header_row_index` is an index into calamine's **trimmed used range**, but every emitted line/column number is an **absolute sheet coordinate**, offset by `range.start()`. On a sheet whose data begins at B6, `header_row_index: 1` means the second row *of the range* (absolute row 7) and the first data row reports `line_number: 8`. Pinned by `test_xlsx_header_row_index_is_relative_to_trimmed_range`, `test_xlsx_offset_sheet_reports_absolute_cell_coordinates`, `test_xlsx_blank_leading_rows_only_shift_line_numbers` and `test_xlsx_blank_leading_columns_only_shift_column_numbers`.
- **XLSX sheet-lookup failures are asymmetric**: `SheetSelection::ByIndex` out of range yields `ParsingFailure::SheetNotFound`, but `ByName` with an unknown name yields `InvalidFormat { details }` carrying calamine's message. Don't assume a missing sheet always produces `SheetNotFound`.
- **SheetSelection has no "First" variant**: Use `ByIndex(0)` instead — `First` was redundant
- **Empty XLSX files are rejected**: an empty sheet returns `ParsingFailure::HeaderNotFound` because there is no header row to extract — this differs from CSV, where an empty file yields `Parsed { rows: [] }`.
<!-- AI-UPDATED: 2026-07-01 - Fixed-width parsing engine added -->
- **Fixed-width has no header row**: each `FixedWidthConfigV1.fields` entry carries its own `header`, so *every* line is a data row — line numbers are plain 1-indexed (no `+2`/`header_row_index` offset). Drop a header/trailer record with a `LineNo` skip config. Pinned by `test_fixed_width_line_numbers_start_at_one` and `test_fixed_width_leading_blank_lines_shift_line_numbers`.
- **Fixed-width `index`/`width` are char counts, not byte offsets**: extraction is `line.chars().skip(index).take(width)`. Fine for ASCII; a multi-byte char earlier in the line shifts every later field. Char-based is deliberate (panic-safe vs. byte slicing splitting a codepoint) — port byte-offset specs carefully. `index` is 0-based, but the emitted `SourceLocation::FileRange` is 1-indexed and inclusive at both ends (`index + 1` .. `index + width`). Pinned by `test_fixed_width_columns_are_absolute_not_relative_to_first_content`.
- **Fixed-width values keep their space padding** (only an out-of-range slice → `None`): configure a `trim` transformation on amount/date fields or validation fails on values like `"  100"` — same reason ValueSkip needs `trim`.

<!-- AI-UPDATED: 2025-07-20 - Transformation split pattern -->
### Adding New Transformation Types
1. Add variant to `TransformationType` enum in common_enums
2. Add config struct to `TransformationConfigData` tagged enum
3. Implement parsing logic in domain/transformation module
4. Add tests with sample file data for the new format

<!-- AI-UPDATED: 2025-07-28 - Generic transformation implementation -->
### Generic Transformation Pattern
<!-- AI-UPDATED: 2025-10-28 - Added order_id_column requirement -->
- **Purpose**: Demo-friendly transformation with configurable column mappings and direction values
- **Config Type**: Uses direct struct embedding: `Generic(GenericTransformationConfig)`
- **Column Mapping**: `GenericColumnMapping` specifies which CSV columns contain date, amount, currency, direction, order_id
- **Required Columns**: All five fields must be present - date, amount, currency, direction, order_id
- **order_id Column**: Must be mapped in config as it's now a main field (not stored in metadata)
- **Direction Mapping**: `GenericDirectionMapping` defines which values map to credit/debit (e.g., "payment" → credit)
- **Metadata Storage**: All CSV columns automatically stored in staging entry metadata for reconciliation rules
- **Error Handling**: Follows standard line-by-line validation pattern with error collection

### Splitting Existing Transformation Types
- **When to Split**: If single transformation handles multiple transaction types with different processing modes
- **Pattern**: Create separate enum variants (e.g., a combined settlement type → `SettlementConfirmation` + `SettlementTransaction`)
- **Implementation**: Split function logic but preserve common field extraction and validation patterns
- **Validation**: Each variant should validate only fields relevant to its transaction types
- **Processing Mode**: Assign appropriate StagingEntryProcessingMode (Confirmation vs Transaction)

### Migrating File Formats
<!-- AI-UPDATED: 2025-07-20 - COBOL to CSV migration pattern -->
- **When to Migrate**: When file format changes from one structure to another (e.g., COBOL to CSV)
- **Pattern**: Update field extraction logic while preserving business logic and metadata structure
- **Implementation**: Change parsing from fixed-position to delimiter-based, update field count validation
- **Backward Compatibility**: Consider versioning if both formats need to coexist temporarily
- **Testing**: Verify all field mappings and ensure no data loss during format transition

### Debugging Failed Transformations
1. Check `transformation_history.data` for failure details and stage
2. Look for file encoding issues (must be UTF-8)
3. Verify `transformation_config.is_active = true` for execution
4. Check ingestion_history_id exists in external system

### Transformation ID Tracking
<!-- AI-UPDATED: 2025-07-22 - Added transformation_id and transformation_history_id to staging entries -->
- **Purpose**: Links staging entries back to their originating transformation for audit trails
- **Implementation**: Two fields in staging_entry table and API models
  - `transformation_id`: NOT NULL - identifies the transformation config used
  - `transformation_history_id`: NULLABLE - identifies the specific execution instance
- **Usage**: 
  - Transformation execution: Both fields populated with actual values
  - Direct API creation: Only `transformation_id` required, `transformation_history_id` set to None
- **Database**: Both columns added to staging_entries table with proper indexing
- **API**: `StagingEntryCreateRequest` includes required `transformation_id` field
- **Pattern**: Pass both `config.transformation_id` and `history.transformation_history_id` to all transformation functions
- **Future**: `transformation_history_id` designed for eventual conversion to `metadata_id`

### Transformation Function Signatures
<!-- AI-UPDATED: 2025-11-03 - TransformationExecutor trait updated for V2 database access -->
- **Breaking Change**: `TransformationExecutor` trait now requires `state: &AppState` parameter for V2 database access
  - **V1 Functions**: Added `_state: &AppState` parameter (unused, prefixed with underscore)
  - **V2 Functions**: Use `state: &AppState` for metadata schema database lookups
  - **Generic Function**: Does not use state parameter, marked as `_state: &AppState`
- **Parameter Order**: Standard order is `(file_data, transformation_id, transformation_history_id, state, encryption_client)`
- **Implementation**: Updated `TransformationExecutor` trait to pass AppState through call chain
- **Transformation Functions**: All transformation functions updated with state parameter
- **Usage**: Functions receive actual history ID from transformation execution context
- **Critical**: Must update function signatures when adding new transformation types

### Performance Considerations
- Transformation configs filtered by `is_active` and `ingestion_id` (indexed)
- History queries use composite indexes on profile_id + status + created_at
- Batch insertion used for staging entries (not individual inserts)

### Testing Transformation Logic
- Use `TransformationExecutor` trait for isolated testing
- Mock file storage interface for file retrieval testing
- Test both success and partial failure scenarios with invalid lines
- Verify staging entry IDs are properly tracked in success data

<!-- AI-UPDATED: 2025-09-02 - Added profile_id and account_id validation -->

### Profile and Account Validation
- **Creation Validation**: Transformation config creation validates profile_id and account_id existence
  - `find_business_profile_by_id()` validates profile_id exists
  - `find_account_by_profile_id_and_account_id()` validates account_id exists and belongs to profile_id
  - Validation failures return `ValidationError` with descriptive messages
  - This prevents creation of transformation configs with invalid/orphaned profile/account references
  - Added new `ValidationError` variant to `TransformationError` enum for consistent error handling

## Relationships with Other Domains

### With Ingestion System
- **Trigger**: Ingestion calls execute endpoint with file_path
- **Contract**: Must provide ingestion_id and ingestion_history_id
- **Dependency**: Assumes ingestion validated file accessibility

### With Metadata Validation
<!-- AI-UPDATED: 2025-10-30 - V2 metadata schema integration completed for unique fields -->
- **V2 Implementation**: `TransformationConfigV2` includes `metadata_schema_id` field
- **Schema Association**: V2 configs link to metadata schemas for validation and unique field extraction
- **Current State**: Unique field extraction from metadata schema is fully implemented
- **Future Work**: Full schema-driven transformation logic to be completed in subsequent PRs
- **Validation**: V2 config creation validates metadata schema exists for the requesting profile
- **Unique Field Integration**: V2 transformation configs extract unique field from metadata schema's unique constraint

### With Reconciliation  
- **Output**: Produces staging entries for reconciliation processing
- **Tracking**: Staging entry IDs stored in transformation success data
- **Flow**: transformation → staging entries → reconciliation rules → final entries

### With Accounts
- **Isolation**: Each transformation config tied to specific account_id
- **Access**: Account context used for staging entry creation and validation

### With Business Profiles
- **Scoping**: All transformation operations filtered by profile_id in JWT
- **Multi-tenancy**: Profile isolation maintained throughout transformation pipeline

## API Response Patterns

<!-- AI-UPDATED: 2025-07-24 - TransformationResultData pattern implementation -->
- **TransformationResultData**: API-friendly representation of transformation results
  - Maps internal `TransformationData` to simplified `TransformationResultData` enum
  - `Processed` variant: Includes counts and errors for client consumption
  - `Failed` variant: Includes failure_stage and error_message for debugging
  - Pattern mirrors `IngestionConnectionData` approach from ingestion domain
- **TransformationDataApiExt**: Extension trait provides `get_result_data()` conversion method
  - Handles versioning: `TransformationData::V1(...)` → `TransformationResultData`
  - Used in `TransformationHistoryResponse.data` field alongside original internal data
- **Response Structure**: `TransformationHistoryResponse` includes both fields:
  - `data`: Optional original internal data (for detailed analysis)
  - `result_data`: Optional API-friendly data (for client consumption)
  - Clients can choose appropriate field based on use case

## Encryption and Performance Optimizations

<!-- AI-UPDATED: 2025-08-13 - Staging entries metadata encryption and batch processing -->
### Metadata Encryption Implementation
- **Database Schema Change**: `staging_entries.metadata` column changed from JSONB to BYTEA for encrypted storage
- **Merchant-Specific Keys**: Each merchant gets isolated encryption keys via `Identifier::ReconMerchant(merchant_id)`
- **Storage Format**: All encrypted metadata stored as `v1:base64` format for consistency across encryption strategies
- **Critical**: `merchant_id` column added to staging_entries table for encryption key isolation and security

### Batch Processing Pattern (3-Pass Optimization)
- **Pass 1**: Validate all CSV rows and collect valid data (no encryption)
- **Pass 2**: Batch encrypt all collected metadata using `encrypt_batch_json_values()`
- **Pass 3**: Create staging entries with encrypted metadata using index-based lookup
- **Performance**: Eliminates O(n) encryption calls, achieving 10-40x performance improvement for large files
- **Pattern Applied**: All transformation functions use this approach

### Encryption Function Signatures
- **Updated Pattern**: All transformation functions now require `encryption_client: &external_services::ApplicationEncryptionClient` parameter
- **Merchant Encryptor**: Use `encryption_client.get_encryptor_for_merchant(merchant_id.to_string())` for key isolation
- **Batch Operations**: Use `FxHashMap<String, serde_json::Value>` with index keys for efficient batch encryption
- **Error Handling**: Encryption failures propagate as `TransformationError::InternalServerError`

### Batch Decryption Implementation
- **Merchant Validation**: All staging entries in batch must belong to same merchant for security
- **UUID-Based Lookup**: Uses proper uuid mapping instead of fragile key-value pairing
- **Usage**: `list_staging_entries_by_profile_id` uses batch conversion to eliminate O(n) decryption calls

## History with Configuration Pattern

<!-- AI-UPDATED: 2025-07-24 - TransformationConfigHistoryView pattern -->
- **TransformationConfigHistoryView**: Efficient joined query for history with config data
  - Single database call instead of multiple separate queries for history + config
  - Used in `find_transformation_history_with_config_by_id()` and list endpoints
  - Provides transformation name and config context alongside history data
- **Database Query Pattern**: Inner join between transformation_history and transformation_configs
  - Join condition: `transformation_history.transformation_id = transformation_configs.transformation_id`
  - Filtered by profile_id for tenant isolation in list operations
  - Supports all existing filters (status, account_id, time_range, etc.)
- **API Enhancement**: Endpoints now return config context with history data
  - `GET /transformations/history/{id}` - Returns history with associated config
  - `GET /transformations/history` - Lists history with config data
  - Response includes `transformation_name` from config for better UX
- **Performance**: Optimized queries use single join instead of N+1 query pattern
  - Critical for list operations with many history entries
  - Maintains indexing on frequently filtered fields (profile_id, status, created_at)

<!-- AI-UPDATED: 2025-09-10 - Transformation-specific history query implementation -->

## Configuration History Tracking

- **Transformation config responses now include `last_transformed_at` timestamp**:
  - `TransformationConfigResponse` has new `last_transformed_at` field alongside existing fields
  - Populated from most recent transformation history entry for that specific config
  - Uses `list_transformation_history_by_transformation_id(&config.transformation_id, Some(1))` pattern
- **History queries are transformation-specific, not profile-level**:
  - `list_transformation_configs_by_profile_id` fetches history per transformation config
  - Each config gets its own timestamp from most recent execution for that transformation
  - Pattern mirrors ingestion domain implementation for consistency
- **Database query implementation**:
  - `list_by_transformation_id` filters by `transformation_id` with `order by created_at DESC`
  - Uses `Some(1)` limit for performance when only latest timestamp needed
  - Maintains existing ordering and filtering patterns for consistency
- **Utility method extensions**:
  - Added `to_response_with_history()` method alongside existing `to_response()`
  - Backward compatibility: `to_response()` sets `last_transformed_at: None`
  - Forward compatibility: New method accepts optional timestamp parameter

<!-- AI-UPDATED: 2025-11-17 - New metadata retrieval endpoint added -->
## Metadata Retrieval API

- **GET /transformations/configs/{transformation_id}/metadata endpoint**:
  - Retrieves metadata schema associated with a transformation config
  - Uses JWT Bearer authentication for tenant isolation (profile_id from auth context)
  - Returns complete `MetadataSchemaResponse` with schema configuration
  - Validates transformation config exists and belongs to authenticated user's profile
  - Validates metadata schema exists and belongs to authenticated user's profile
- **Implementation pattern**:
  - Transformation config provides `metadata_schema_id` via `get_metadata_schema_id()` method
  - Database lookup validates both transformation and schema belong to requesting profile
  - Error handling: `TransformationConfigNotFound` or `MetadataSchemaNotFound` as appropriate
  - Response conversion uses standard `to_api_response()` pattern for metadata schema

<!-- AI-UPDATED: 2025-11-21 - Metadata schema ID column extraction from JSON -->
## Database Schema Evolution

### Metadata Schema ID Column Migration
- **metadata_schema_id column added** to transformation_configs table as VARCHAR(64) NOT NULL
- **Database normalization**: Moved metadata_schema_id from JSON config blob to dedicated column
- **Foreign key semantics**: Explicit relationship between transformation configs and metadata schemas
- **API response impact**: `metadata_schema_id` now exposed directly in TransformationConfigResponse
- **V2 configs only**: Migration applies to V2 transformation configs (V1 configs deprecated)

<!-- AI-UPDATED: 2026-06-07 - Exposed/searchable metadata for reporting -->
<!-- AI-UPDATED: 2026-06-11 - project_metadata moved to recon utils; un-validated create now derives subsets; engine still calls exposed_keys() via build_search_entry_keys -->
## Exposed & Searchable Metadata (Reporting)

- **Why**: A reporting Lambda runs directly on the DB but cannot group/aggregate on the encrypted `metadata` (BYTEA). So per-field, the metadata schema can mark a metadata field as exposed (stored **unencrypted**) and optionally searchable.
- **Config**: `ReconMetadataFieldConfigV1.is_exposed: Option<MetadataExposeConfig { is_searchable: bool }>` (common_types + api_models). `None` = not exposed; `Some { is_searchable: false }` = exposed; `Some { is_searchable: true }` = exposed + searchable. The Option-of-struct encoding makes the invariant **searchable ⊆ exposed ⊆ metadata** structural (no cross-field validation needed). It's `#[serde(default)]` so old stored schemas (JSONB) deserialize as not-exposed — **no schema migration**.
- **Where the subsets are computed (the important part)**: in `RowValidator::validate_row` (`domain/metadata_schema/data_validator.rs`), inside the same loop that validates each metadata field — where the field's `is_exposed()` config, its authoritative key, AND its value are all in hand. The subsets are built from the **same field-name keys** as `metadata` and stored on `EntryData.exposed_metadata` / `.searchable_metadata`. **Why here, not by re-projecting later**: validated metadata is keyed by **field_name** (`fetch_field_name().to_field_path_without_metadata_prefix()`), NOT identifier. Re-deriving a key set elsewhere risks using the wrong key (identifier vs field_name) and silently producing empty subsets when they differ.
- **`exposed_keys()` returns FIELD NAMES, not identifiers**: `MetadataSchemaData::exposed_keys()` → `ExposedKeys { exposed, searchable }` (in `common_types/src/metadata_schema.rs`) is used ONLY by the sites that DON'T go through `validate_row` and must re-derive subsets from already-stored/raw metadata — the two `core/staging_entry.rs` sites (`update_staging_entry` (**edit**) and the un-validated `create_staging_entry`) plus the recon engine's `build_search_entry_keys` (see reconciliation notes). It deliberately returns field names to match the metadata key convention; `project_metadata(metadata, keys)` filters a JSON object to those keys. **`project_metadata` now lives in `recon/src/utils/metadata_schema.rs`** (moved out of `common_types`; `exposed_keys()`/`ExposedKeys` stay in `common_types`).
- **Staging side**: `staging_entry` gained two **unencrypted** JSONB columns `exposed_metadata` + `searchable_metadata` (NOT NULL default `'{}'`). `domain/transformation.rs` just carries the already-computed subsets from `EntryData` through `TransformationStagingEntryData` into `StagingEntryNew` (full `metadata` is still batch-encrypted; subsets ride alongside as plain JSON). All three create paths populate the subsets: `create_staging_entry_with_validation` reads them off the validated entry; both the **edit** path (`update_staging_entry`) and the un-validated `create_staging_entry` re-derive via `exposed_keys()` + `project_metadata` after resolving the `transformation_id`'s config → schema — `create_staging_entry` falls back to empty `{}` only when that config/schema lookup fails.
- **Gotcha**: `exposed_metadata`/`searchable_metadata` are plain `serde_json::Value` (JSONB), never wrapped in `Encryption`. Only `metadata` is encrypted.
