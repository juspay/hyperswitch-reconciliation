# Metadata Validation - Transformation Rules

> **Note**: Also read `AGENT-NOTES.md` in this folder for the broader validation architecture.

## Scope
<!-- AI-UPDATED: 2026-07-06 - DefaultValue now available on string -->
- **`Trim`** is available on every datatype: `string`, `number`, `minorunit`, `majorunit`, `datetime`, `enum`, `currency`, `balance_direction`.
- **`DefaultValue`** is available on `string`, `currency`, and `balance_direction`. The remaining datatypes have no default support — don't assume symmetry when wiring new rules.
- **`JsonExtract`** is available on every datatype. Extracts values from JSON strings using RFC 6901 JSON Pointer notation (e.g., `/pspReference`, `/address/state`, `/items/0`).
- **`Negate`** is only available on `majorunit` today. Flips the sign on the raw string before parsing (`"-2750.00"` ↔ `"2750.00"`).
- **`Absolute`** is available on both `minorunit` and `majorunit`. Strips a leading `-` so negative values become positive; non-negative values are untouched. Distinct from `Negate` on `majorunit`: `Negate` always flips the sign, `Absolute` collapses to non-negative. See "`Absolute` Behavior and Ordering" below.
- **`ReplaceChar`** is available on `majorunit` and `string`. Replaces or deletes occurrences of a single character anywhere in the raw string. On `majorunit` it runs _before_ the decimal-`delimiter` swap and strips locale grouping separators (`1,000,000` → `1000000`); on `string` it normalizes identifier/reference values so equality joins line up across sides (`TXN-2026-00123` → `TXN202600123`). See "`ReplaceChar` Behavior and Ordering" below.
- **`StartsWith`** is only available on `balance_direction` today. Replaces the raw value with the configured `then` string when it starts with `prefix`, otherwise the configured `otherwise` string — operators pick whatever sentinels their `credit_values` / `debit_values` already contain. See "`StartsWith` Behavior and Ordering" below.
- **`Regex`** is available on `string` and `datetime` (the latter covers both metadata datetime fields and the main `effective_at` column). Extracts the configured capture group (or the full match) from the raw cell value. See "`Regex` Behavior and Edge Cases" below.
- **`Truncate`**, **`AddDuration`**, **`SubtractDuration`** live on `datetime` only, in a **separate** `post_parse_transformations` list (not `transformation_rules`). They operate on the parsed `PrimitiveDateTime`. Used to compose hour/day/month/year grouping keys for equality joins across related datetimes.

## Transform Runs Before Parse, Not After
<!-- AI-UPDATED: 2026-04-23 - Added PostParseTransformer trait alongside ValueTransformer -->
- Every datatype's `validate_value` runs the `transformation_rules` loop on the raw `Option<String>` **before** calling `parse::<f64>()` / `Currency::parse()` / `Date::parse()` / enum-mapping lookup.
- Why: CSV ingestion yields strings, so `Trim` on `" 123 "` must fire before `f64::parse` rejects it as `InvalidType`. Running trim after parse would be a no-op.
- **When adding a new pre-parse rule**: it must be expressible on `String`. `ValueTransformer::DataType` is always `String` even when `ValueValidator::DataType` is `f64` / `MinorUnit` / `PrimitiveDateTime` / `Currency`.
- **When adding a new post-parse rule** (only supported on `datetime` today): implement `PostParseTransformer` with the parsed type as `DataType` (e.g., `PrimitiveDateTime`). See the DateTime section below.

## Rules Run In Array Order
<!-- AI-UPDATED: 2026-04-14 - Order-sensitive pipeline -->
- The executor walks `transformation_rules` left-to-right, mutating `&mut Option<String>` in place. Earlier rules see the raw CSV value; later rules see the output of all previous ones.
- Trim is idempotent and always safe to put first. Default semantics are covered in the next section — Trim does **not** make `DefaultValue` fire on whitespace-only cells.

## `DefaultValue` Fires Only on `None`, Not Empty Strings
<!-- AI-UPDATED: 2026-07-06 - Same None-only rule now applies to string -->
- `DefaultValue` substitutes only when the incoming value is `None`; `Some("")` is left untouched. For `currency` / `balance_direction` the parser then fails with `InvalidCurrency` / `InvalidBalanceDirection`; for `string` the empty value flows on to the validation rules (so a `MinLength` rule still rejects it).
- This is intentional — it prevents typos like `" "` from being silently masked by defaults. Same rule across all three datatypes.
- Verified by `test_currency_default_value_not_applied_when_empty` (`datatypes/currency.rs`) and `test_string_default_value_not_applied_when_empty` (`datatypes/string.rs`). Don't change the semantics without updating those tests.

## Main Fields Duplicate Datatype `transformation_rules`
<!-- AI-UPDATED: 2026-04-14 - Two-place coupling that is easy to miss -->
- Main CSV `currency` / `balance_direction` / `amount` / `effective_at` fields go through `…FieldConfigV1` wrappers, which build a `…Config` via `From` and delegate to the datatype's `validate_value`. The wrappers don't re-implement parsing.
- **Consequence**: `transformation_rules` must live on **both** `…ConfigV1` (in `common_types::metadata_schema::datatypes`) **and** `…FieldConfigV1` (in `common_types::metadata_schema::fields`), and be copied across in the `From<&…FieldConfigV1>` impl.
- Forgetting the field wrapper: metadata-field gets transformations, main CSV column silently doesn't.

## Where api↔common Conversions Live
<!-- AI-UPDATED: 2026-04-14 - Codebase convention, not a Rust idiom -->
- **Never** add `impl From<ApiType> for CommonType` in `api_models` for transformation-rule enums.
- Use `MetadataSchemaDataApiExt` / `MetadataSchemaDataDieselExt` trait impls in `crates/recon/src/utils/metadata_schema.rs` — all conversions live in one file so reviewers audit a single place.
- Call sites use `.map(|r| r.to_api_response())` / `.map(|r| r.to_diesel_schema())`, **not** `.map(Into::into)` — grep-friendly.

## API Enum vs common_types Enum Name Clash
<!-- AI-UPDATED: 2026-04-14 - Import aliasing required in mapper -->
- Both sides share short names (e.g. `CurrencyTransformationRule`). api_models is flat-tagged (`#[serde(tag = "transformation_rule_type")]`); common_types is versioned (`V1(…V1::Variant { … })`).
- In `utils/metadata_schema.rs`, import the api_models side as `…Api` (e.g. `CurrencyTransformationRule as CurrencyTransformationRuleApi`).
- **Wire format**: JSON uses the flat api_models shape (e.g. `{"transformation_rule_type": "trim"}` or `{"transformation_rule_type": "default_value", "currency": "USD"}`) — the versioned tag must never leak into request/response bodies. DateTime `post_parse_transformations` use a distinct tag key, `post_parse_rule_type` (e.g., `{"post_parse_rule_type": "truncate", "precision": "start_of_hour"}`), to keep the two enums cleanly separated in wire format just as they are in Rust.

## `JsonExtract` Behavior and Edge Cases
<!-- AI-UPDATED: 2026-04-20 - JsonExtract transformation rule implementation -->
- **Input**: Raw cell value containing JSON string (e.g., `{"pspReference": "ABC123", "amount": 100}`)
- **Pointer**: RFC 6901 JSON Pointer path (e.g., `/pspReference`, `/nested/key`, `/array/0`)
- **Output Types**:
  - Strings → extracted as-is
  - Numbers → converted to string (e.g., `100` → `"100"`)
  - Booleans → converted to string (e.g., `true` → `"true"`)
  - Arrays → serialized to JSON string (e.g., `["a","b"]` → `"[\"a\",\"b\"]"`)
  - Objects → serialized to JSON string (e.g., `{"k":"v"}` → `"{\"k\":\"v\"}"`)
  - Null → becomes `None` (not an error)
- **Errors**:
  - `JsonParseFailed`: Cell value is not valid JSON
  - `JsonKeyNotFound`: Pointer path doesn't exist in the JSON
- **Shared Implementation**: `json_extract_value()` in `transformations/json_extract.rs` used by all datatypes
- **Order Sensitivity**: JsonExtract should typically run after Trim (to handle whitespace in JSON strings) and before parsing validations

## `Negate` Behavior and Ordering
<!-- AI-UPDATED: 2026-04-22 - Sign-flip rule for sign-flipped amount  -->

- **Scope**: `majorunit` only. Symmetric with `Trim` — mutates the raw `Option<String>` _before_ `f64::parse`.
- **Semantics**: If value starts with `-`, strips the prefix. Otherwise prepends `-`. Empty strings and `None` are no-ops.
- **Always `Trim` first**: Leading whitespace like `" -2750.00"` would otherwise miss the `-` prefix check. Canonical config: `[{trim}, {negate}]`.
- **Pair with `positive_only` deliberately**: After `Negate`, a legitimate negative-in-CSV becomes positive and passes `PositiveOnly`. Conversely, a _positive_-in-CSV becomes negative and fails `PositiveOnly` — this is intentional and the defense against a corrupt source file that flips signs.
- **Anti-pattern**: Don't use `Negate` on a datatype where the sign has semantic meaning beyond direction (e.g., signed currency values where `-` marks refund vs payment). For those, model the direction through `balance_direction` instead.

## `Absolute` Behavior and Ordering
<!-- AI-UPDATED: 2026-05-06 - Extended Absolute to majorunit -->

- **Scope**: `minorunit` and `majorunit`. Mutates the raw `Option<String>` _before_ `i64::parse` (minorunit) / `f64::parse` (majorunit).
- **Semantics**: If value starts with `-`, strips the prefix. Non-negative values, empty strings, and `None` pass through unchanged. Same one-arm shape on both datatypes — only the downstream parser differs.
- **`Absolute` vs `Negate` on `majorunit`**: both rules live on `majorunit`. `Negate` always flips the sign (`"-x"` ↔ `"x"`); `Absolute` collapses to non-negative (`"-x"` → `"x"`, `"x"` → `"x"`). Pick `Absolute` when you want unsigned magnitudes regardless of source sign; pick `Negate` when the source file consistently flips signs. Stacking both is a smell — the second rule wins and the first is dead code.
- **Always `Trim` first**: Leading whitespace like `" -2750"` would otherwise miss the `-` prefix check. Canonical config: `[{trim}, {absolute}]`.
- **Pair with `positive_only` to enforce the invariant**: After `Absolute`, every value is non-negative, so `PositiveOnly` becomes a no-op for legitimate input but still rejects pre-transform pathologies (e.g., a value of `-` alone, which strips to empty and then fails parsing as `InvalidType`).
- **Use case**: Source files that express debits as negative amounts but the downstream model wants unsigned magnitudes paired with an explicit `balance_direction`. Strip the sign here; carry direction in the `balance_direction` field.
- **Anti-pattern**: Don't use `Absolute` to "clean up" a column where sign carries meaning the schema doesn't model elsewhere — you'll silently drop information. Model the direction explicitly first.

## `ReplaceChar` Behavior and Ordering
<!-- AI-UPDATED: 2026-09-07 - Extended ReplaceChar to the string datatype -->

- **Scope**: `majorunit` and `string`. Mutates the raw `Option<String>` — on `majorunit` _before_ the built-in decimal-`delimiter` → `.` swap and `f64::parse`, on `string` before the `min_length` / `max_length` validation rules. Identical semantics on both: one shared helper (`transformations/replace_char.rs`), generic over `Option<String>`, so further datatypes can opt in without reimplementing.
- **`from: char`, `to: Option<char>`**: `to: None` **deletes** the character; `to: Some(c)` replaces it with `c`. Deletion is the primary use case (stripping grouping separators), which a bare `char` can't express — hence `Option`.
- **`mode`**: `ReplaceMode::All` acts on every occurrence; `ReplaceMode::Single { occurrence, from_end }` targets exactly one. `occurrence` is **0-indexed**, counted from the left (`from_end: false`) or from the right (`from_end: true`) — so `{ occurrence: 0, from_end: true }` is the last match.
- **No-ops (never errors)**: a `None` value (blank cell), a `from` character not present, and an out-of-range `Single.occurrence` all leave the value unchanged. Same permissive stance as `Negate` / `Absolute` — a misconfigured replace should not abort a row.
- **Composition with `delimiter` is the point**: set `from` to the **grouping** char and leave `delimiter` as the **decimal** char; they never collide because the rule runs before the delimiter swap.
  - US (`delimiter=Dot`): `[{trim}, {replace_char, from:",", to:null, mode:{all}}]` → `"1,000,000.00"` → `1000000.00`.
  - EU (`delimiter=Comma`): `[{trim}, {replace_char, from:".", to:null, mode:{all}}]` → `"1.000.000,00"` → `"1000000,00"` → delimiter swap → `1000000.00`.
  - Same char for grouping **and** decimal: `[{replace_char, from:",", to:".", mode:{single, occurrence:0, from_end:true}}, {replace_char, from:",", to:null, mode:{all}}]` → `"1,234,56"` → `"1234.56"`.
- **On `string`, it fills the gap `strip_prefix` / `strip_suffix` / `regex` leave**: those only touch the ends or extract a slice — none of them can substitute *inside* a value. Canonical use: `TXN-2026-00123` (PSP side) vs `TXN202600123` (internal side) → `[{trim}, {replace_char, from:"-", to:null, mode:{all}}]` on the PSP side makes the two join on `equals`. Swapping separators (`/` → `_`) is the same rule with `to` set.
- **`string` keeps the permissive stance, even though its neighbours don't**: `strip_prefix` / `strip_suffix` / `regex` on `string` fail loudly when they don't match, but `replace_char` no-ops. Deliberate — the two datatypes share one helper and diverging their semantics would be worse than the local inconsistency.
- **Available everywhere `Vec<StringTransformationRule>` is**: string metadata fields (`FieldTypeConfig::String`), the `order_id` main field (`OrderIdFieldConfigV1`), and transformation-config value-skip rules (`ValueSkipConfig`). Wiring the variant lit up all three at once.
- **Always `Trim` first** when the source has padding — same ordering rule as `Negate` / `Absolute`.
- **Multibyte-safe**: occurrence targeting uses `char_indices`, so a multi-byte char earlier in the string does not shift the target.
- **Wire format**: `{"transformation_rule_type":"replace_char","from":",","to":null,"mode":{"mode_type":"all"}}`; Single → `"mode":{"mode_type":"single","occurrence":0,"from_end":true}`. Same flat shape on both datatypes.

## `StartsWith` Behavior and Ordering
<!-- AI-UPDATED: 2026-05-05 - Prefix-routed substitution rule with configurable then/otherwise outputs -->

- **Scope**: `balance_direction` only. Mutates the raw `Option<String>` _before_ the `credit_values` / `debit_values` lookup.
- **Semantics**: `Some(s)` becomes `Some(then)` if `s.starts_with(prefix)`, otherwise `Some(otherwise)`. `None` passes through (so a missing cell stays `None` — pair with `DefaultValue` if you need a fallback). Empty `Some("")` does **not** start with any non-empty prefix, so it routes through the `otherwise` branch.
- **Canonical config**: `credit_values: ["credit"]`, `debit_values: ["debit"]`, `transformation_rules: [{trim}, {starts_with, prefix: "CR", then: "credit", otherwise: "debit"}]`. Source values like `"CR-12345"` route to `Credit`; anything not starting with `CR` routes to `Debit`. The `then` / `otherwise` strings are arbitrary — pick whatever your `credit_values` / `debit_values` already contain (avoids needing a sentinel set just for this rule).
- **No implicit polarity**: swapping `then` and `otherwise` swaps the routing — there's no built-in "match prefix → credit" assumption. Pinned by `test_balance_direction_starts_with_then_and_otherwise_can_swap_routing`.
- **Always `Trim` first**: Leading whitespace would otherwise miss the prefix check. Same ordering rule as `Negate` / `Absolute`.
- **Config-time validation**: empty `prefix` is rejected at schema-save time with `EmptyStartsWithRulePrefix` (an empty prefix would route every non-`None` row through the `then` branch — a likely-misconfigured config that should fail loudly). Empty `then` / `otherwise` are allowed (you may legitimately want to route to the empty string and let the parent `validate_value` raise `InvalidBalanceDirection`).
- **Wire format**: `{"transformation_rule_type": "starts_with", "prefix": "CR", "then": "credit", "otherwise": "debit"}`.
- **Anti-pattern**: If the source column has more than two distinct prefixes (e.g., `CR`/`DR`/`RV`) you cannot fold them down to a single boolean branch — model with `enum` mappings or with explicit multi-element `credit_values` / `debit_values` instead.

## `Regex` Behavior and Edge Cases
<!-- AI-UPDATED: 2026-08-11 - Regex extended to the datetime datatype -->
- **Input**: Raw cell value as a string. `None` passes through unchanged.
- **Pattern**: Any `regex` crate pattern (Rust `regex` crate syntax — **no lookaround, no backreferences** by design). Patterns are compiled on every row invocation; `regex::Regex::new` failure surfaces as a runtime error.
- **Extraction semantics**: `group: Some(n)` returns capture group `n`; `group: None` (or omitted) returns `captures.get(0)`, the whole match. A pattern with no groups is therefore usable with `group` left unset.
- **Errors**:
  - `RegexCompileFailed { pattern, reason }`: pattern did not compile (should be caught at config-save time via `validate_config`, but also surfaced at runtime for defense-in-depth).
  - `RegexGroupOutOfRange { pattern, group, capture_count }`: config-time only — `group` exceeds the number of capture groups in the pattern.
  - `UnmatchedRegex { pattern }`: pattern is valid but the input has no match. Intentionally an error to parallel `StripPrefix`/`StripSuffix` — missing data should fail loudly, not silently pass through.
- **Wire format escaping**: Regex backslashes must be escaped in JSON — `\d` in a pattern is written `"\\d"` in the JSON request body (e.g., `{"transformation_rule_type": "regex", "pattern": "Order_ID:\\s*(\\d+)", "group": 1}`).
- **Config-time validation**: `StringConfigV1::validate_config` and `DateTimeConfigV2::validate_config` compile each `Regex` pattern (and range-check `group`) so malformed configs fail at schema-save time, not on the first row processed. Main-field wrappers (`OrderIdFieldConfigV1`, `EffectiveAtFieldConfigV2`) don't re-run rule validation — that's a pre-existing gap shared by every transformation rule, not specific to `Regex`.
- **Shared implementation**: `regex_extract_value()` in `transformations/regex_extract.rs`. Wired to `StringTransformationRuleV1` and `DateTimeTransformationRuleV1`; the helper is generic over `Option<String>` so other datatypes can opt in without reimplementing.
- **Order sensitivity**: Regex typically runs after `Trim` (strip noise whitespace) and before parsing validations. If combined with `StripPrefix`/`StripSuffix`, put `Regex` last so earlier rules shape the string first.
- **On `datetime`, regex only slices — it does not reformat**: the extracted substring is still parsed against the configured `date_time_format`. Extracting `22/06/2026` under a `year_month_day` + `hyphen` format fails with `InvalidFormat`; the capture group must match the declared format exactly. Pinned by `regex_extracted_value_still_parsed_against_configured_format`.
- **Canonical `datetime` use case**: `effective_at` cells that wrap the date in prose — `"Totals for Payment on 22/06/2026"` with `transformation_rules: [{regex, pattern: "(\\d{2}/\\d{2}/\\d{4})", group: 1}]` and `date_format: {order: day_month_year, delimiter: slash}`.
- **Performance note**: Pattern compilation is per-invocation today (same pattern as `JsonExtract`'s `serde_json::from_str`). Hot rows with identical patterns would benefit from caching a compiled `Regex` — revisit if profiling shows it matters.

## DateTime has Two Rule Lists: `transformation_rules` (pre-parse) and `post_parse_transformations` (post-parse)
<!-- AI-UPDATED: 2026-04-23 - Split-list design + PostParseTransformer trait dispatch -->
- The DateTime config has two separate rule lists that run around `PrimitiveDateTime::parse`. The pre-parse invariant for `transformation_rules` (see "Transform Runs Before Parse, Not After" above) still holds — post-parse operations are a named, explicit second list.
- **`transformation_rules: Vec<DateTimeTransformationRule>`** runs first, on `Option<String>`, before parse. Variants: `Trim`, `JsonExtract`, `Regex`. Same semantics as every other datatype's `transformation_rules`.
- **`post_parse_transformations: Vec<DateTimePostParseTransformationRule>`** runs after parse, on `PrimitiveDateTime`. Dedicated enum — the compiler rejects putting `Truncate` / `AddDuration` / `SubtractDuration` into `transformation_rules` and vice versa. No runtime no-op matching.
- **Dispatch is trait-based, mirroring pre-parse**: `DateTimePostParseTransformationRule` implements `PostParseTransformer { type DataType = PrimitiveDateTime; fn apply_post_parse_transformation(&self, value: &mut Self::DataType) }`. The validator call site reads `post_parse_rule.apply_post_parse_transformation(&mut parsed_datetime)` — same ergonomics as `transformation.transform_value(&mut value)` for pre-parse. Trait impls live in `datatypes/datetime.rs` alongside the `ValueTransformer` impls for the pre-parse rules; the underlying helpers (`apply_truncation`, `apply_duration_addition`, `apply_duration_subtraction`) live as `pub fn` in `transformations/datetime_post_parse.rs` (same layout convention as `json_extract.rs`).
- **`Truncate { precision }`** rounds the datetime down to the start of the specified precision. Variants: `start_of_hour`, `start_of_day`, `start_of_month`, `start_of_year`. Example: `2026-03-25 14:30:45` with `start_of_hour` → `2026-03-25 14:00:00`.
- **`AddDuration { duration: { value, unit } }` / `SubtractDuration { ... }`** shifts the datetime by a magnitude + unit. Units: `minutes`, `hours`, `days`. Uses `time::Duration` + `checked_add` / `checked_sub`, so overflow surfaces as `InvalidDateTimeConfig` rather than panicking. Example: `2026-03-25 23:45:00` + `1 hour` → `2026-03-26 00:45:00` (day rollover is correct).
- **Composability is the point**: classic hour-grain join uses `post_parse_transformations: [{add_duration, 1 hour}, {truncate, start_of_hour}]` on one side and `post_parse_transformations: [{truncate, start_of_hour}]` on the other. Both sides produce identical `PrimitiveDateTime` values — a plain `equals` match rule joins on them. Month-level groupings use `Truncate { start_of_month }`. Negative offsets via `SubtractDuration`.
- **Output stays `PrimitiveDateTime`, not string**: the match layer compares parsed datetimes directly. No string serialization format to agree on across sides.
- **Error model**: truncation/arithmetic failures return `RuleError::InvalidDateTimeConfig { details }` with a short reason. Input parse failures surface via `InvalidFormat` from the parse step (between the two phases).
- **Scope**: `post_parse_transformations` is available on both `DateTimeConfigV2` (metadata datetime fields) and `EffectiveAtFieldConfigV2` (main `effective_at` column). In practice it's used on derived metadata fields, not on the canonical timestamp.
