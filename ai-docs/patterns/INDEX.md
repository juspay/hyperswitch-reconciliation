# Patterns Index

Quick reference for implementation patterns. Read the full pattern file when implementing.

## API Patterns

- **ADD-ENDPOINT.md** - Add new API endpoint with route→core→db architecture
  - When: Creating any new REST endpoint
  - Key: Layer separation, response types, error handling

- **ADD-AUTH.md** - Add JWT authentication to endpoints  
  - When: Endpoint needs merchant/profile validation
  - Key: JwtAuth levels, path parameter matching

- **API-DOCUMENTATION.md** - Document endpoints with OpenAPI
  - When: Any new or updated endpoint
  - Key: utoipa annotations, schema examples

## Data Patterns

- **HANDLE-MONEY.md** - Work with monetary amounts
  - When: Any operation involving money
  - Key: AmountType for API, MinorUnit for DB, currency handling

- **DATABASE-QUERY.md** - Build database queries with Diesel
  - When: Querying with filters or complex conditions, or paginating lists
  - Key: Async diesel, optional filters, cursor/keyset pagination, proper error handling

- **ADD-DATABASE-FIELD.md** - Add new database columns
  - When: Extending existing tables
  - Key: Migration files, model updates, schema regeneration

- **ADD-STATUS-ENUM.md** - Add new status enums
  - When: Creating new state machines or enum constraints
  - Key: Diesel enum, migrations, trait implementations

- **SOFT-DELETE.md** - Implement soft deletion
  - When: Adding soft delete to any entity
  - Key: discarded_at timestamp, status preservation

## Code Quality Patterns

- **ERROR-HANDLING.md** - Handle errors with proper context
  - When: Any operation that can fail
  - Key: change_context, to_not_found_response, custom errors

- **TESTING.md** - Write integration tests
  - When: Testing any new functionality
  - Key: TestApp setup, data builders, transaction rollback

- **CUSTOM-SERDE.md** - Custom serialization for datetime and complex types
  - When: Working with PrimitiveDateTime in APIs
  - Key: ISO8601 format, UTC normalization, optional handling

- **ADD-STATUS-ENUM.md** - Add new status enums
  - When: Creating new state machines
  - Key: Diesel enum, migrations, trait implementations

- **VERSIONED-ENUMS.md** - Create backward-compatible versioned enums
  - When: Enums that may evolve over time with stored data
  - Key: Tagged unions, JSON storage, version migration, backward compatibility

## Architecture Patterns

- **MODULE-STRUCTURE.md** - Organize new modules
  - When: Creating new feature modules or external service integrations
  - Key: Route/Core/DB separation, external services consolidation

- **ENCRYPTION.md** - Implement data encryption
  - When: Adding encryption to sensitive data fields
  - Key: TypeEncryption trait, ApplicationEncryptionInterface, strategy pattern

- **RESPONSE-TYPES.md** - Create API responses
  - When: Defining new API responses
  - Key: ApplicationResponse, DTOs, never expose DB models
