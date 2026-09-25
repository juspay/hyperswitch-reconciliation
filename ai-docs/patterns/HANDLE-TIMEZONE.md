# Handle Timezones

**When**: any work involving IANA timezones or per-profile local time.

**Scope today**: a profile's timezone is stored and echoed back only. No conversion exists yet.

## The type: `common_types::timezone::Timezone`

A newtype over `chrono_tz::Tz`, used unchanged at every layer (API, domain, diesel).
Constructing it is the validation - `FromStr`/`Deserialize` reject unknown zones.

```rust
let timezone: Timezone = "Asia/Kolkata".parse()?;
let default = Timezone::UTC;          // also `Timezone::default()`
timezone.to_string();                 // "Asia/Kolkata"
```

- **Don't write a timezone validator** - it would be dead code.
- **Only `common_types` depends on `chrono-tz`.** Use the wrapper; don't name `chrono_tz` elsewhere.
- Serde is hand-written (the `chrono-tz` `serde` feature is off), so the wire form is the plain IANA name.
- No accessor for the inner `Tz` exists yet - add one with the first real consumer.

**Why a wrapper**: the orphan rule forbids diesel impls on foreign `Tz`, and chrono-tz has no
diesel feature. Diesel plumbing is `common_utils::impl_to_sql_from_sql_text!(Timezone)`, same
shape as `FieldPathKey`/`MinorUnit`/`Email`.

## Behaviour

- **Invalid request input**: rejected by `web::Json` before core runs - actix's plain-text 400,
  not the `ApiErrorResponse` envelope (same as any invalid enum, e.g. `Currency`).
- **Corrupt stored value**: fails in `FromSql`, so the whole `business_profile` row fails to
  deserialize. Never defaulted to UTC - that would silently put entries in the wrong bucket.
- **Aliases are not canonicalized**: `Asia/Calcutta` round-trips as-is and `!=` `Asia/Kolkata`.
  Pinned by `update_business_profile_preserves_deprecated_timezone_aliases`; if it fails,
  stored values need migrating.

## Storage

- Timestamps stay naive UTC. Timezone is presentation/bucketing metadata only.
- `business_profile.timezone` is `VARCHAR(64) NOT NULL DEFAULT 'UTC'`; the app always writes an
  explicit value, the default is only a safety net.
- **No SQL constraint on purpose**: `CHECK` can't query `pg_timezone_names`, and a seeded FK
  table would be a second tzdb that can drift from chrono-tz's. The Rust type enforces validity.

## Future: conversion

Nothing converts to local time yet. When needed:

- Put helpers in one module next to `DomainBusinessProfile` in `recon_domain_models` (adds a
  direct `chrono` dep; the rest of the workspace uses `time`).
- All helpers must be fallible - DST gaps/overlaps make local times non-existent or ambiguous.
- Never reuse a resolved offset across arithmetic (a DST day is 23/25 hours); re-resolve each step.

Current naive-UTC code that will need it:
- `crates/recon/src/utils/overview.rs` - bucket truncation
- `crates/recon/src/domain/metadata_schema/transformations/datetime_post_parse.rs` - `StartOf*`
- `crates/common_types/src/recon_rule/aging_config.rs` - business-day counting
