# Pattern: Custom Serialization/Deserialization

## When to Use
- Serializing time types (PrimitiveDateTime) to/from specific formats
- Ensuring consistent date/time formatting across APIs
- Handling optional datetime fields
- Custom JSON serialization for complex types

## Quick Example
```rust
use common_utils::custom_serde;
use time::PrimitiveDateTime;

#[derive(Serialize, Deserialize)]
pub struct ApiModel {
    // Required datetime field with ISO8601 format
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    
    // Optional datetime field
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}
```

## Available Custom Serde Modules

### 1. ISO8601 DateTime (Recommended)
```rust
// For required PrimitiveDateTime fields
#[serde(with = "common_utils::custom_serde::iso8601")]
pub effective_at: PrimitiveDateTime,

// For optional PrimitiveDateTime fields  
#[serde(with = "common_utils::custom_serde::iso8601::option")]
pub discarded_at: Option<PrimitiveDateTime>,
```

**Features:**
- Formats as: `2024-01-01T00:00:00.000Z` (with milliseconds)
- Always converts to/from UTC
- Handles leap seconds gracefully
- Schema annotation: `#[schema(value_type = String, example = "2024-01-01T00:00:00Z")]`

### 1b. ISO8601 Microsecond DateTime (Cursor pagination only)
```rust
#[serde(with = "common_utils::custom_serde::iso8601_micros")]
pub effective_at: PrimitiveDateTime,
```
**Use only for keyset pagination cursor fields** (e.g. `EffectiveAtSortField`, `StagingEntryEffectiveAtSortField`). Postgres `Timestamp` stores microseconds; the query builder compares the decoded cursor back against the column with `.eq()`. The millisecond `iso8601` format truncates sub-millisecond digits, breaking that equality check and causing rows to skip/duplicate on page navigation. Don't use it for plain display fields — `iso8601` is still correct there.

### 2. Unix Timestamp
```rust
// For required timestamp fields
#[serde(with = "common_utils::custom_serde::timestamp")]
pub created_at: PrimitiveDateTime,

// For optional timestamp fields
#[serde(with = "common_utils::custom_serde::timestamp::option")]
pub deleted_at: Option<PrimitiveDateTime>,
```

**Features:**
- Serializes as Unix timestamp (seconds since epoch)
- Useful for systems expecting numeric timestamps

### 3. JSON String Serialization
```rust
// For complex types that need to be stored as JSON strings
#[serde(with = "common_utils::custom_serde::json_string")]
pub complex_data: SomeComplexType,
```

**Features:**
- Serializes any type to a JSON string
- Useful for storing complex objects in string fields

## Implementation Details

The custom serde implementations are located in `crates/common_utils/src/custom_serde.rs`.

Key design decisions:
- **UTC Normalization**: All timestamps are converted to UTC during serialization
- **Consistent Format**: ISO8601 with millisecond precision for all datetime fields
- **Error Handling**: Parse errors are properly propagated with descriptive messages
- **Leap Second Support**: The implementation correctly handles leap seconds

## Common Patterns

### API Request/Response Models
```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRequest {
    pub account_id: String,
    pub amount: FloatMajorUnit,
    
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub effective_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetResponse {
    pub id: String,
    pub status: Status,
    
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub processed_at: Option<PrimitiveDateTime>,
}
```

### Database Model Conversion
```rust
impl From<DbModel> for ApiResponse {
    fn from(model: DbModel) -> Self {
        Self {
            id: model.id,
            // PrimitiveDateTime fields automatically serialize correctly
            // due to the serde attributes on ApiResponse
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
```

## Testing Considerations

When testing custom serde implementations:

```rust
#[test]
fn test_datetime_serialization() {
    let model = ApiModel {
        created_at: time::macros::datetime!(2024-01-01 12:00:00),
    };
    
    let json = serde_json::to_string(&model).unwrap();
    assert!(json.contains("2024-01-01T12:00:00.000Z"));
    
    let deserialized: ApiModel = serde_json::from_str(&json).unwrap();
    assert_eq!(model.created_at, deserialized.created_at);
}
```

### 4. Query Parameter Parsers
```rust
// For parsing comma-separated query parameters
#[serde(default, deserialize_with = "parse_comma_separated")]
#[param(example = "pending,posted", value_type = Vec<EntryStatus>)]
pub status: Option<Vec<EntryStatus>>,
```

**Features:**
- Parses comma-separated values from query strings
- Supports any type that implements `FromStr`
- Returns `Option<Vec<T>>` for optional filtering
- Example: `?status=pending,posted` → `Some(vec![Pending, Posted])`

**Swagger/OpenAPI Best Practices:**
<!-- AI-UPDATED: 2025-01-16 - Swagger type specification guidance -->
```rust
// ✅ CORRECT: Use actual enum type in value_type
#[param(example = "pending,posted", value_type = Vec<EntryStatus>)]

// ❌ INCORRECT: Don't use generic String types
#[param(example = "pending,posted", value_type = Vec<String>)]

// For optional fields, use #[serde(default)] instead of required = false
#[serde(default)]
#[param(value_type = BalanceDirection)]
pub entry_type: Option<BalanceDirection>,

// For flattened structs (e.g., TimeRange), ensure the nested type derives IntoParams
#[serde(flatten, default)]
#[param(value_type = TimeRange)]
pub time_range: Option<TimeRange>,
```

**Implementation:**
```rust
pub mod query_parsers {
    pub fn parse_comma_separated<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Debug + std::fmt::Display + std::error::Error,
    {
        let input: Option<&str> = serde::Deserialize::deserialize(deserializer)?;
        input
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .map(|item| item.parse::<T>().map_err(serde::de::Error::custom))
                    .collect::<Result<Vec<T>, _>>()
            })
            .transpose()
    }
}
```

## Gotchas

<!-- AI-CREATED: 2025-01-07 - Initial documentation -->

- **Schema Annotations**: Always include `#[schema(value_type = String)]` for OpenAPI documentation
- **UTC Conversion**: Input timestamps in any timezone are converted to UTC
- **Millisecond Precision**: `iso8601` uses 3 decimal places for seconds; `iso8601_micros` uses 6 (only for pagination cursor fields, to match Postgres microsecond precision — see above)
<!-- AI-UPDATED: 2026-07-11 - time crate's well-known Iso8601 formatter silently misrounds subsecond digits -->
- **Never format subseconds with `Iso8601::<FORMAT_CONFIG>`**: the well-known formatter (`time::format_description::well_known::Iso8601` + `Config::set_time_precision`) computes fractional-second digits via float math and is off by 1 for ~20% of values (e.g. stored millisecond `501` → formatted `"500"`; stored microsecond `133520` → `"133519"`). Both `iso8601` and `iso8601_micros` now use an explicit `format_description!("...[subsecond digits:N]...")` instead, which extracts digits with integer arithmetic and is exact. This wasn't cosmetic: for `iso8601_micros`, the corrupted cursor no longer equals the DB row it came from, so the keyset pagination `.eq()` tie-break silently dropped that row from every page forever. Any new subsecond-precision format in this file must use `[subsecond digits:N]`, not the well-known `Iso8601` config API — verify with an exhaustive sweep test over the full digit range (see `custom_serde::tests`), a handful of spot values won't reliably catch this.
- **Option Handling**: Use the `::option` submodule for Option<T> fields
- **Database Storage**: PrimitiveDateTime maps to TIMESTAMP columns (without timezone)
- **Leap Seconds**: Properly handled (e.g., "2023-12-31T23:59:60.000Z")
<!-- AI-UPDATED: 2025-01-10 - Query parameter parsing -->
- **Query Parameters**: Use `parse_comma_separated` for filtering with multiple values
- **Trim Whitespace**: Parser automatically trims spaces around comma-separated values
- **Parse Errors**: Invalid enum values in query params return 400 Bad Request
