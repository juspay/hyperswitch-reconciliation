# Metadata Validation - Agent Notes

> **Note**: Also read `TRANSFORMATION-RULES.md` in this folder for the pre-parse transformation pipeline, `DefaultValue` semantics, and api↔common_types conversion conventions.

## Database Schema

### metadata_schemas Table
<!-- AI-UPDATED: 2025-09-03 - Foundation implementation -->
- **Primary Key**: `id` with format `meta_sch_{nanoid(10)}`
- **Schema ID**: `schema_id` with format `meta_sch_main_{nanoid(10)}`
- **Data**: `schema_data` stores versioned JSONB schema definitions

### Key Indexes

- `idx_metadata_schemas_profile_id` - For profile-scoped lookups
- `idx_metadata_schemas_schema_id` - For schema-id based lookups
- `idx_metadata_schemas_account_id` - For account-id based lookups
- `uniq_idx_metadata_schemas_schema_id_version` - Unique index on schema-id and version based lookups

## Common Gotchas

### ID Generation
<!-- AI-UPDATED: 2025-09-03 - Nanoid patterns -->
- Table primary key: `meta_sch_{nanoid(10)}` (auto-generated)
- User-facing identifier: `meta_sch_main_{nanoid(10)}` (user-provided)
- Never use auto-increment - always nanoid format

### Diesel Type Integration
<!-- AI-UPDATED: 2025-09-03 - Enum export issue -->
- Must export new enums in `common_enums/lib.rs` diesel_exports section
- Schema file references via `common_enums::diesel_exports::*`

### Versioned Data Structures
<!-- AI-UPDATED: 2025-09-05 - Current implementation -->
- `MetadataSchemaData` is versioned enum in common types (`V1`)
- API models use simplified enum with semantic variants (`BasicSchemaData`)
- All types implement `ToSchema` for OpenAPI docs

## Field Schema Structure
<!-- AI-UPDATED: 2025-09-05 - Current implementation -->
- **identifier**: User-friendly field name (e.g. "customer_name")
- **field_name**: Field name for data access (e.g. "customer_name")
- **validation_rules**: Vec of validation rules (NotNull, MaxLength, MinLength)
- **field_type**: String or Number
- **description**: Mandatory field description

## Database Queries
<!-- AI-UPDATED: 2025-09-05 - Available queries -->
- `find_by_id()` - Find schema by primary key
- `find_by_id_and_profile_id()` - Profile-scoped access for security
- `list_by_profile_id()` - List all schemas for a profile
- `insert()` - Create new schema
- `create_metadata_schema_version()` - Version increment pattern

## API Models Pattern  
<!-- AI-UPDATED: 2025-10-30 - Unique constraint structure simplified -->
- **Simplified Structure**: API models use simplified enum with semantic variants
- **BasicSchemaData**: Current variant with fields and single unique_constraint
- **Breaking Change**: `unique_constraints: Vec<UniqueConstraint>` → `unique_constraint: UniqueConstraint`
- **Conversion Layer**: `MetadataSchemaDataApiExt` trait converts from common types to API
- **Scalable Design**: Ready for future variants (StandardSchemaData, ProfessionalSchemaData, etc.)

<!-- AI-UPDATED: 2026-08-10 - Composite unique constraints added -->
- `UniqueConstraintTypeV1::CompositeField(CompositeUniqueField)` lets a constraint span multiple fields; `SingleField` still covers the one-field case
- Composite fields are sorted before joining, so config order doesn't affect the resulting key/value
- `CompositeDelimiter` (`Pipe` or `None`) and the composite-join helpers (`build_composite*`) live in `common_types::composite_field` — a top-level module, not nested under any one domain, since both `metadata_schema` and `recon_rule` types (`CompositeUniqueField`, `CompositeSearchIdentifier`, `CompositeGroupingField`) need them
- `UniqueConstraintExt::extract_unique_value()` (in `recon`, not common_types) returns raw `Vec<(EntryField, Option<String>)>` — never a joined string or an error. Deciding a missing value is an error, and how to join present values, is left to the caller

## Conversion Layer
<!-- AI-UPDATED: 2025-09-05 - Trait-based conversion -->
- **MetadataSchemaDataApiExt**: Converts common types to API models
- **Location**: `crates/recon/src/utils/metadata_schema.rs`
- **Pattern**: Follows same approach as staging_entry and transaction utils
- **No From Traits**: Uses explicit conversion methods for clarity (see `TRANSFORMATION-RULES.md` for the specific trap when adding new rule types)

## Query Layer Patterns
<!-- AI-UPDATED: 2025-09-05 - Current implementation -->
- **Profile Scoping**: All queries filter by profile_id for multi-tenant security
- **Error Handling**: Use `DatabaseError::Others` with `db_metrics::track_database_call`
- **Query Pattern**: Follow `generics::db_metrics::track_database_call` pattern for all operations
- **Type Safety**: Use consistent types throughout database layer

## Architecture Overview
<!-- AI-UPDATED: 2025-09-05 - New modules and architecture -->

### Core Modules Added
- **`crates/recon/src/core/metadata_schema.rs`** - Business logic for CRUD operations
- **`crates/recon/src/routes/metadata_schema.rs`** - HTTP route handlers with OpenAPI docs
- **`crates/storage_impl/src/metadata_schema.rs`** - Database interface trait and implementation
- **`crates/recon/src/errors/metadata_schema.rs`** - Domain-specific error types
- **`crates/recon/src/utils/metadata_schema.rs`** - API/Diesel model conversion traits

### Data Model Architecture
- **`crates/common_types/src/metadata_schema.rs`** - Core domain types with versioning
- **`crates/api_models/src/metadata_schema.rs`** - External API request/response models
- **`crates/diesel_models/src/metadata_schema.rs`** - Database entity models

### API Endpoints Implemented
```
POST   /metadata_schemas           - Create new schema
GET    /metadata_schemas           - List schemas with filters
GET    /metadata_schemas/{id}      - Get schema by ID
PUT    /metadata_schemas/{id}      - Update schema (creates new version)
```

### Filter System Architecture
- **`ListMetadataSchemasFilters`** in common_types for query parameters
- **Query Support**: schema_id, account_id, time_range (start_time/end_time)
- **Database Layer**: Modified `list_by_profile_id()` with dynamic filter application
- **Validation**: Time range validation with custom error handling

### Versioning System
- **Schema Evolution**: Updates create new records with incremented version numbers
- **ID Strategy**: New schema versions get new primary key but same schema_id
- **`MetadataSchemaVersionUpdate`** enum for different update types
- **`new_versioned()` method** in MetadataSchemaNew for version creation

### Test Infrastructure
- **`crates/recon/tests/api/metadata_schema_test.rs`** - Comprehensive integration tests
- **Test Coverage**: CRUD operations, filtering, authorization, profile isolation
- **Filter Testing**: Complex combinations of schema_id + account_id + time_range
- **Authorization Testing**: Profile-based access control validation

## Mapping Module Architecture
<!-- AI-UPDATED: 2024-12-19 - Added mapping functionality for schema-driven data filtering -->

### Domain Types Added
<!-- AI-UPDATED: 2026-08-18 - rows carry a SourceRecord, not a uuid -->
- **`ParsedRow`** - Input type: `{ source: SourceRecord, fields: HashMap<String, LocatedValue<Option<String>>> }`. **No uuid** — position is carried by `source`, and each value carries its own `SourceLocation`.
- **`MappedRow`** - Output type: `{ source: SourceRecord, mapped_fields: HashMap<String, LocatedValue<Option<String>>> }`, filtered by schema
- **Extension Trait Pattern** - `MetadataSchemaMapper` trait adds mapping functionality to schema types

### Key Implementation Details
- **Field Filtering**: Only schema-defined identifiers are kept in output
- **Missing Field Handling**: Schema fields not found in raw data are set to None
- **Direct Identifier Lookup**: Raw fields use identifier as key (no metadata. prefix)
- **Extension Trait**: Clean API allowing `schema.map_row(parsed_row)` — **one row at a time**; the collection-level `map_entries` was removed, traversal belongs to the caller

### Pipeline Architecture
```
Raw CSV Data → ParsedRow (all fields) → MappedRow (schema fields only) → Validation Phase
```

### Critical Design Decisions
<!-- AI-UPDATED: 2024-12-19 - Field naming and lookup strategy -->
- **Field Name Migration**: Renamed `field_path` to `field_name` across all types for clarity
- **Direct Identifier Mapping**: Raw CSV columns must match schema identifiers exactly  
- **No Internal Prefixing**: Mapping phase uses direct identifier lookup, validation phase handles prefixing
- **Schema-Driven Filtering**: Only fields defined in schema survive the mapping phase

## Specialized Field Types Architecture
<!-- AI-UPDATED: 2025-09-15 - Extended types with versioned business logic -->

### Main Field Types with Versioned Configuration
<!-- AI-UPDATED: 2025-10-28 - Added order_id as main field -->
- **Required Fields**: currency, amount, effective_at, balance_direction, order_id (always present)
- **Versioned Pattern**: Each main field has versioned config enums (e.g., CurrencyFieldConfig::V1)
- **API Simplification**: Users only provide identifiers for main fields, full config for metadata fields
- **Storage Evolution**: Uses versioned enums throughout for future extensibility
- **order_id Field**: Added as main field (not in metadata) with MaxLength(64) validation
  - Stored in dedicated VARCHAR(64) column in staging_entries and entries tables
  - MaxLength validation prevents 5xx database errors on insert
  - Must be included in all GenericColumnMapping configurations
  - All test cases must provide order_id in mapped rows

### BalanceDirection Field Configuration
<!-- AI-UPDATED: 2025-09-15 - Credit/debit values in versioned types -->
- **User-Configurable Values**: credit_values and debit_values Vec<String> in both API and storage types
- **Storage Location**: In BalanceDirectionFieldConfigV1 and BalanceDirectionConfigV1
- **API Location**: In BalanceDirectionSchemaConfig and FieldType::BalanceDirection
- **Configuration Mapping**: Direct mapping between API and storage types via conversion traits
- **API Input Required**: Users must specify which strings represent credit vs debit

### MinorUnit Validation Rules
<!-- AI-UPDATED: 2025-09-15 - Currency precision handling with versioning -->
- **Type Support**: MinorUnitValidationRule with MinValue, MaxValue, and PositiveOnly variants
- **Storage Pattern**: MinorUnitValidationRule::V1(MinorUnitValidationRuleV1)
- **Import Required**: `use common_types::units::MinorUnit;` in api_models
- **Conversion Layer**: All validation rule conversions handle MinorUnit correctly
- **Precision Benefit**: Avoids floating-point precision issues in currency calculations

### DateTime Field Configuration
<!-- AI-UPDATED: 2026-01-13 - Configurable datetime formats -->
- **Optional Format Config**: `EffectiveAtSchemaConfig` and `FieldType::DateTime` have `date_time_format: Option<DateTimeFormat>`
- **Version Migration**: V1 configs use None (ISO-8601 default), V2 configs use Some(DateTimeFormat) with custom formats
- **FieldType Breaking Change**: `DateTime` changed from unit variant to struct variant - update all pattern matches
- **Format Validation**: DateTimeConfigV2 validates format at schema creation via `build_format_description()`
  - Prevents storing invalid datetime configs that would fail at data ingestion time
  - Uses time crate's format parser to validate delimiter and component combinations

#### Fractional Seconds & Timezone Offset Support
<!-- AI-UPDATED: 2026-04-14 - ISO 8601 parsing for sources -->
- **Timezone stored as UTC, not preserved**: When `TimeFormatConfig.timezone` is Some, the validator parses via `OffsetDateTime` then normalizes to UTC before downcasting to `PrimitiveDateTime`. The original offset is *lost* at ingest — downstream code sees naive UTC only. This is intentional: the domain type is timezone-naive and the hyperswitch-wide `custom_serde::iso8601` serializer appends `Z`, so the frontend localizes correctly.
- **`FractionalSeconds::Microseconds { separator }`**: Single variant for now — only 6-digit precision is needed by the current sample files. `separator` reuses the existing `separator` enum (`.`) rather than hard-coding a `.`, so future sources using `,` or other separators can be added without touching the format builder. Add `Milliseconds` / `Nanoseconds` variants when a source file actually needs them.
- **`separator::TLiteral`**: Needed specifically as `separator_from_date` for ISO 8601 (`2026-03-25T03:50:40`). The `time` crate format DSL treats any non-bracketed character as literal, so no escaping is required — but a bare `T` is ambiguous in text, hence the explicit variant.
- **Adding fields to `TimeFormatConfig` breaks struct literals**: Any new required/optional field forces updates to all struct-literal call sites, even test fixtures (E0063). The existing `field_tests/datetime_test.rs` has 5 rstest cases that must be updated in lockstep.

## Type System Gotchas
<!-- AI-UPDATED: 2025-09-15 - Conversion layer complexities with versioning -->

### API ↔ Common Types Conversion Patterns
- **Trait-Based Mapping**: Uses MetadataSchemaDataApiExt and MetadataSchemaDataDieselExt traits
- **Bidirectional Conversion**: Both to_api_response() and to_diesel_schema() must handle all versioned fields
- **Version Handling**: Each enum level requires pattern matching on version variants (V1, future V2, etc.)
- **Field Structure Alignment**: API flattened structure maps to deeply nested versioned storage types
- **Constructor Calls**: When converting API → Common Types, create proper versioned enum instances

### Versioned Type System Evolution
- **Enum Wrapping Pattern**: Every type uses versioned enums (e.g., FieldTypeConfig::V1(...))
- **Future Compatibility**: V2, V3 variants can be added without breaking stored schemas
- **Pattern Matching**: All match statements must handle version variants explicitly
- **Storage Migration**: Database can contain multiple versions simultaneously

### Test Update Requirements
- **Template Updates**: All test helper functions need updated field structures with versioned types
- **Required Fields**: Tests must provide credit_values and debit_values arrays
- **FieldType Syntax**: Use struct initialization: `FieldType::String { required: true, validation_rules: vec![] }`
- **Version Construction**: Create versioned types: `CurrencyFieldConfig::V1(CurrencyFieldConfigV1 { ... })`

### Compilation Error Patterns
- **Missing Fields**: E0063 errors when new fields added to structs
- **Type Mismatches**: MinorUnit vs f64 in validation rule conversions  
- **Pattern Exhaustiveness**: E0027 errors when destructuring patterns miss new fields or version variants
- **Constructor Arity**: Function signature changes require updating all call sites
- **Version Mismatches**: Must match on version enums before accessing inner data

## Schema-Based Data Mapping Architecture
<!-- AI-UPDATED: 2025-09-15 - Added mapping functionality for schema-driven data filtering -->

### Mapping Module Overview
- **Location**: `crates/recon/src/domain/metadata_schema/mapper.rs`
- **Purpose**: Filters raw CSV data through metadata schema to extract only defined fields
- **Data Flow**: ParsedRow (all fields) → MappedRow (schema fields only) → Validation Phase

### Core Mapping Types
<!-- AI-UPDATED: 2026-08-18 - MappingOrchestrator renamed to RowMapper; uuid removed -->
- **ParsedRow**: Input type carrying `source: SourceRecord` and a `fields` HashMap of `LocatedValue<Option<String>>`
- **MappedRow**: Output type carrying `source` and a `mapped_fields` HashMap, filtered by schema
- **Extension Trait Pattern**: `RowMapper` trait adds mapping functionality to schema types

### Key Implementation Details
- **Field Filtering**: Only schema-defined identifiers are kept in output
- **Missing Field Handling**: Schema fields not found in raw data are set to None
- **Direct Identifier Lookup**: Raw fields use identifier as key (no metadata. prefix)
- **Extension Trait**: Clean API allowing `schema.map_row(parsed_row)`

### Mapping Trait Hierarchy
- **RowMapper**: Top-level trait, maps exactly one row (`map_row`)
- **FieldsConfigMapper**: Maps individual field configurations 
- **MetadataSchemaMapper**: Maps specific field types (currency, amount, etc.)
- **Versioned Implementation**: All traits handle versioned types throughout the mapping chain

### Pipeline Integration
```rust
// Raw CSV Data → ParsedRow (all fields) → MappedRow (schema fields only) → Validation Phase
let outcome = parsing_config.parse(file_content);   // FileParsingOutcome, never a Result
let mapped_row = schema.map_row(parsed_row);        // per row, inside the engine loop
// Further processing with only schema-defined fields
```

### Performance Characteristics
- **O(n*m) Complexity**: n entries × m schema fields
- **HashMap Lookups**: Direct field access by identifier
- **Memory Efficient**: Creates new HashMap only for mapped fields, not copying all data
- **Schema-Driven**: Processing scales with schema size, not raw data size

## Validation Architecture (Third Level)
<!-- AI-UPDATED: 2025-09-24 - Comprehensive validation implementation -->

### Validation Pipeline Flow
```rust
// Complete 3-level transformation
Raw CSV Data → ParsedRow → MappedRow → ValidatedEntryData
```

### Staging Entry V2 Integration
<!-- AI-UPDATED: 2025-10-30 - Integration with staging entry V2 endpoint -->
- **Production Integration**: Validation pipeline now integrated with `POST /staging_entries/v2` endpoint
- **End-to-End Flow**: API request → Schema validation → Data mapping → Data validation → Staging entry creation
- **Error Propagation**: Validation errors surface as HTTP 400 with detailed error messages
- **Security Integration**: Schema access validated via JWT profile ownership before processing
- **Encryption Layer**: Validated metadata encrypted using merchant-specific keys before storage

### Fail-Fast vs Multi-Error Behavior
- **Main Fields (Fail-Fast)**: amount, currency, balance_direction, effective_at
  - Validation stops at first error for any main field
  - Only one error returned per row when main field fails
  - Required for system integrity - all main fields must be valid
- **Metadata Fields (Multi-Error)**: All fields in metadata_fields array
  - Validation continues through all metadata fields even if some fail
  - Multiple errors can be returned for a single row
  - Allows users to see all validation issues at once

### Field Identifier vs Field Name Distinction
<!-- AI-UPDATED: 2025-09-24 - Critical naming confusion -->
- **identifier**: User's column name in CSV (e.g., "tr_id", "fee")
- **field_name**: Schema field name (e.g., "transaction_id", "fee_amount")
- **Validation Uses**: identifier for error reporting and metadata JSON keys
- **Metadata JSON**: Uses identifier as key, not field_name
- **Common Mistake**: Tests failing because expected metadata uses field_name instead of identifier

## Configuration Validation Architecture
<!-- AI-UPDATED: 2025-10-01 - Separated config validation from data validation -->

### Critical Separation of Concerns
- **Config Validation**: Validates schema definitions themselves (ConfigValidator trait)
- **Data Validation**: Validates actual data against schemas (ValueValidator trait)
- **Module Split**: `validator.rs` → `data_validator.rs` + `config_validator.rs`
- **Import Updates**: All existing imports must reference `data_validator` module

### Config Validation Rules
- **Field Name Uniqueness**: Metadata field names must be unique within schema
- **Non-Empty Validation**: field_name, identifier, description cannot be empty/whitespace
- **Balance Direction**: credit_values and debit_values cannot be empty, no duplicates across both arrays
- **Validation Rules**: No duplicate rule types (e.g., multiple MaxValue rules not allowed)
- **Number Validation**: Values cannot be NaN or infinite

### Module Organization Gotchas
<!-- AI-UPDATED: 2025-10-01 - Import path changes -->
- **Breaking Change**: Renamed `validator.rs` → `data_validator.rs`
- **New Module**: Added `config_validator.rs` with ConfigValidator trait
- **Export Updates**: `mod.rs` exports `RowValidator` from `data_validator` (renamed from `ValidationOrchestrator`; `MappingOrchestrator` likewise became `RowMapper`)
- **All Imports**: Must update from `validator::` to `data_validator::`

### ConfigValidator Implementation Pattern
- **Trait Coverage**: Every schema type implements ConfigValidator
- **Validation Chain**: Top-level schemas validate their child components recursively
- **Early Return**: Config validation fails fast on first error encountered
### Error Type Hierarchy 
- **Two-Level System**: `ConfigValidationError` (top) + `ConfigFieldValidationError` (bottom)

## Transformation Rule Pipeline
<!-- AI-UPDATED: 2026-04-14 - Extracted to TRANSFORMATION-RULES.md -->
- See `TRANSFORMATION-RULES.md` in this folder for pipeline order, trim-before-default rule ordering, main-field delegation, the api↔common_types enum name clash, and where conversions live.

### JsonExtract Implementation Details
<!-- AI-UPDATED: 2026-04-20 - Shared extraction logic for all datatypes -->
- **Location**: `crates/recon/src/domain/metadata_schema/transformations/json_extract.rs`
- **Function**: `json_extract_value(value: &mut Option<String>, identifier: &str)` 
- **Shared Across**: All datatypes (String, Number, Currency, DateTime, Enum, BalanceDirection, MinorUnit, MajorUnit)
- **Integration Pattern**: Each datatype's `ValueTransformer` implementation calls `json_extract::json_extract_value(value, pointer)` when encountering `JsonExtract` rule
- **Null Handling**: JSON `null` values become `None`, not an error — this allows optional fields in JSON structures
- **Complex Types**: Arrays and objects serialize to their JSON string representation using `serde_json::Value::to_string()`, enabling downstream storage of nested structures as strings

### Regex Implementation Details
<!-- AI-UPDATED: 2026-08-11 - Regex extraction rule now wired on datetime too -->
- **Location**: `crates/recon/src/domain/metadata_schema/transformations/regex_extract.rs` (module named `regex_extract` to avoid clash with the `regex` crate name).
- **Functions**: `regex_extract_value(value: &mut Option<String>, pattern: &str, group: Option<usize>)` and `validate_regex_config(pattern: &str, group: Option<usize>)`.
- **Currently wired on**: `StringTransformationRuleV1` and `DateTimeTransformationRuleV1`. The shared helper lives under `transformations::` so other datatypes can opt in without a second implementation — wiring a new datatype is one `ValueTransformer` arm plus one `ConfigValidator` arm.
- **Extraction Semantics**: `group: Some(n)` returns capture group `n`; `group: None` returns the whole match (`captures.get(0)`). A group that matched nothing (optional group in the pattern) surfaces as `UnmatchedRegex`, not `None`.
- **Error Surface**: `RuleError::InvalidConfig { details }` (bad pattern, wrapped compilation error) and `RuleError::UnmatchedRegex { pattern }` (input had no match). No-match is intentionally an error (parallel to `StripPrefix`/`StripSuffix`); if you need optional extraction, use two different rules or a new variant. Note: Runtime regex compilation failures should not occur since patterns are validated at config-save time via `ConfigFieldValidationError::RegexCompileFailed`.
- **Config-Time Check**: `validate_regex_config` compiles the pattern and range-checks `group`, failing fast with `ConfigFieldValidationError::RegexCompileFailed` / `RegexGroupOutOfRange`. Both `StringConfigV1::validate_config` and `DateTimeConfigV2::validate_config` iterate `transformation_rules` to reach it — keep those loops if you add more syntactically-constrained transformation rules.
- **On DateTime, regex slices but does not reformat**: the extracted substring is still parsed against the configured `date_time_format`, so the capture group must match the declared `order`/`delimiter` exactly. A pattern that pulls `22/06/2026` out of `"Totals for Payment on 22/06/2026"` needs `date_format: {order: day_month_year, delimiter: slash}` — the regex cannot rewrite separators.
- **ReDoS**: The `regex` crate guarantees linear-time matching; no catastrophic backtracking concern.

### DateTime Post-Parse Implementation Details
<!-- AI-UPDATED: 2026-04-23 - PostParseTransformer trait + split-list schema for DateTime -->
- **Location**: `crates/recon/src/domain/metadata_schema/transformations/datetime_post_parse.rs` exports `pub fn apply_truncation`, `pub fn apply_duration_addition`, `pub fn apply_duration_subtraction`. All take `&mut PrimitiveDateTime` — these are the computational primitives, not the dispatch layer.
- **Dispatch**: `PostParseTransformer` trait in `data_validator.rs` (sibling to `ValueTransformer`) with signature `fn apply_post_parse_transformation(&self, value: &mut Self::DataType) -> Result<(), Report<RuleError>>`. Impls for `DateTimePostParseTransformationRule` and `DateTimePostParseTransformationRuleV1` live in `datatypes/datetime.rs` and call into the helper module. The validator calls `post_parse_rule.apply_post_parse_transformation(&mut parsed_datetime)` as a trait method.
- **Why `&mut T` not `&mut Option<T>`**: by the time post-parse runs, the value is guaranteed present (we bailed if parse returned `None`). `ValueTransformer` uses `&mut Option<T>` because pre-parse sees raw CSV cells that can be empty.
- **Schema shape**: `DateTimeConfigV2 { format, transformation_rules, post_parse_rules }` and `EffectiveAtFieldConfigV2 { ..., transformation_rules, post_parse_rules }`. Two separate `Vec`s — the enums don't share variants, so the compiler rejects misplacing a rule between phases.
- **Non-versioned payload types**: `TruncationPrecision`, `DateTimeDuration`, `DurationUnit` are leaf payloads consumed by `DateTimePostParseTransformationRuleV1` variants. They're not versioned — the outer `DateTimePostParseTransformationRule::V1(...)` wrapping is the version boundary, same pattern as `pointer: String` inside `JsonExtract`. Additive evolution (new variants) doesn't need a version bump; breaking changes introduce `DateTimePostParseTransformationRule::V2(...)`.
- **Error model**: all three helpers return `RuleError::InvalidDateTimeConfig { details }` on failure (overflow, out-of-range calendar date). Pre-parse errors (invalid format) surface via `ValueError::InvalidFormat` from the parse step between the two phases.

## Exception Management Integration
<!-- AI-UPDATED: 2025-11-18 - Added validate_entry_data endpoint -->
- **New Endpoint**: `POST /exception_management/validate_entry_data`
- **Purpose**: Validates single raw entry data against a transformation's metadata schema
- **Input**: transformation_id + raw_entry_data HashMap<String, Option<String>>
- **Output**: `RowValidationOutcome::{Validated(ValidatedEntryData), Rejected(RowRejection)}` — an outcome enum, not a `Result`. `RowRejection { source_record, field_errors: Vec<LocatedValue<FieldError>> }` renders to the API via `to_api_response()`.
- **Security**: JWT authentication required, transformation config ownership validated
- **Integration**: Uses existing validation pipeline (mapping → validation)
- **Edge Case**: Handles empty string fields as missing required fields for validation
