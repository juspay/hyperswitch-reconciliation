# Ingestion - Agent Notes

## Overview

Ingestion service handles automated file retrieval from external sources (primarily PSPs). It's a webhook-driven system that downloads, validates, and stores files for later transformation.

## Common Gotchas

<!-- AI-UPDATED: 2025-01-21 - Initial ingestion service documentation -->

- Trigger endpoint `/ingestions/{id}/trigger` accepts ANY source type, not just webhooks
  - Webhooks are one trigger source, but system supports API calls, scheduled jobs, etc.
  - Each PSP implements different trigger data parsing (Adyen uses webhook format)
- HMAC validation happens BEFORE file download to prevent unnecessary API calls
- Files are stored with pattern: `ingestions/{ingestion_id}/{history_id}/{filename}`
  - Each processing attempt gets unique history_id for isolation
- Ingestion configs can be inactive - webhook calls to inactive configs return 400
- Basic Auth validation for Adyen uses two different credential sets:
  - `webhook_basic_auth` for incoming webhook validation
  - `report_basic_auth` for outbound file download from Adyen

<!-- AI-UPDATED: 2025-07-21 - Manual file upload feature analysis -->

- Manual ingestion configs support direct file uploads via `/ingestions/{id}/upload` endpoint
  - Upload endpoint ONLY works with `Manual` ingestion config type, not `Psp` configs
  - File size limited to 10MB at multipart form level
  - Empty files are rejected with validation error
- Manual uploads trigger transformation service via HTTP call after successful storage
  - Uses fire-and-forget pattern - upload succeeds even if transformation trigger fails
  - Transformation service URL configured in `transformation_service.base_url`

## Status Flow

<!-- AI-UPDATED: 2025-01-21 - Critical status progression for debugging -->

- Status progression: `pending` → `processing` → `processed`/`failed`
- Failed status includes failure stage: `Parse`, `Download`, or `Storage`
- Raw payload is ALWAYS stored regardless of processing outcome
- `processed_at` timestamp only set on final success, not on processing start

<!-- AI-UPDATED: 2025-07-23 - History update pattern changes -->

- History updates now use specific enum variants instead of generic Update
  - `ProcessingUpdate` - sets status to Processing with trigger_data
  - `ProcessedUpdate` - sets status to Processed with success data
  - `FailedUpdate` - sets status to Failed with failure data
- Each update type automatically sets appropriate timestamps

## Security Implementation

<!-- AI-UPDATED: 2025-01-21 - Security constraints discovered -->

- Adyen HMAC uses hex-decoded secret key with SHA256
- Webhook payload format is strict: must match exact field order for signature validation
- Basic Auth header validation is case-insensitive ("Authorization" or "authorization")
- File download timeout is hardcoded to 5 minutes (300 seconds)

## PSP-Specific Constraints

<!-- AI-UPDATED: 2025-01-21 - Adyen implementation details -->

- Adyen `REPORT_AVAILABLE` events:
  - Download URL comes from `reason` field, not a separate field
  - Filename extracted from `pspReference` field
  - Must validate event type before attempting download
- Empty notification items array is a validation failure, not a parse error

<!-- AI-UPDATED: 2025-07-23 - Adyen config structure refactor -->

- Adyen config now uses dedicated `AdyenIngestionConfig` struct wrapped in enum
  - Breaking change: config access pattern changed from direct fields to enum variant
  - Example: `config.hmac_secret` → `match config { Adyen(cfg) => cfg.hmac_secret }`

## Database Design

<!-- AI-UPDATED: 2025-01-21 - Schema gotchas -->

- Config stored as JSONB allows version evolution (currently V1)
- History table stores both `trigger_data` (parsed) and `raw_payload` (original)
- No foreign key between ingestion_history and ingestion_configs for operational flexibility
- Profile_id is denormalized into history table for direct queries

<!-- AI-UPDATED: 2025-07-23 - Raw payload type system -->

- `raw_payload` is now a typed enum `RawPayload` with variant-specific data
- Adyen variant stores full webhook JSON, Manual variant has no payload
- Breaking change from `serde_json::Value` - migrations may need type conversion

<!-- AI-UPDATED: 2025-09-03 - Audit logging with versioning implementation -->

- **Audit Logging with Versioning**: ingestion_history now uses transaction-style versioning pattern
  - `id`: Primary key (unique per version), format: `ing_hist_{nanoid}`
  - `ingestion_history_id`: Business identifier (constant across versions), format: `ing_hist_main_{nanoid}`
  - `version`: Version number starting at 1, increments with each status change
  - **Critical**: Previous versions marked as `status=Discarded` with `discarded_at` timestamp
  - `discarded_status` preserves original status before discarding (Pending, Processing, etc.)
- **Versioning Triggers**: Every status transition creates a new version
  - Pending → Processing → Processed/Failed each create separate version records
  - Use `create_ingestion_history_version_and_discard_previous()` for atomic operations
  - **Never manually update status** - always create new version to maintain audit trail
- **Query Impact**: Multiple records per ingestion with same `ingestion_history_id`
  - Latest version has `discarded_at=NULL`, previous versions have timestamps
  - Use `ingestion_history_id` filter in list API to see all versions of same ingestion
  - Primary key queries with `id` return specific version, not latest

## Integration Points

<!-- AI-UPDATED: 2025-01-21 - Service boundaries -->

- File storage uses external_services interface - supports S3 and filesystem
- No direct database access patterns - all operations go through store layer
<!-- AI-UPDATED: 2026-10-05 - Trigger auth is done by the ingestion context -->
- JWT auth for CRUD operations. The trigger route is `NoAuth` at the route; each ingestion context authenticates in `parse_payload` (see Trigger Auth below)

<!-- AI-UPDATED: 2025-07-21 - Transformation service integration implemented -->

- Transformation service integration via HTTP API calls after successful ingestion
  - Manual uploads automatically trigger transformation via POST to `/transformations/{ingestion_id}/{history_id}/trigger`
  - Uses reqwest client for service-to-service communication within same application
  - Fire-and-forget pattern - ingestion succeeds even if transformation trigger fails

## When Working Here

<!-- AI-UPDATED: 2025-01-21 - Development workflow gotchas -->

- Always test with realistic webhook payloads - parsing is strict
- File download errors should update history with failure details
- Profile isolation enforced in list operations but not in trigger processing
- Admin API key required for config mutations, JWT sufficient for reads

<!-- AI-UPDATED: 2025-09-02 - Added profile_id and account_id validation -->

- **Profile and Account Validation**: Ingestion config creation validates profile_id and account_id existence
  - `find_business_profile_by_id()` validates profile_id exists
  - `find_account_by_profile_id_and_account_id()` validates account_id exists and belongs to profile_id
  - Validation failures return `ValidationError` with descriptive messages
  - This prevents creation of ingestion configs with invalid/orphaned profile/account references

<!-- AI-UPDATED: 2025-07-23 - Error handling patterns -->

- Error types are now more specific with enum variants per stage
  - `ParseError` with subtypes like `JsonParseError`, `TimeRangeValidation`
  - `DownloadError` with subtypes like `ResponseReadError`
  - `StorageError` with subtypes like `UploadFailed`
- Generic error messages replaced with structured error context

## Architecture Patterns

<!-- AI-UPDATED: 2025-07-23 - Context abstraction pattern -->

- `IngestionContext` abstraction unifies webhook and manual upload flows
  - `create_context_from_webhook()` for PSP webhooks
  - `create_context_for_manual_upload()` for file uploads
  - Both flows converge to single `process_ingestion()` function
- Core processing logic is now PSP-agnostic, context handles specifics

## Configuration History Management

<!-- AI-UPDATED: 2025-07-24 - History tracking patterns from config-history-list feature -->

- Ingestion config responses now include `last_synced_at` timestamp from most recent history entry
  - Requires join with ingestion_history table to get sync status
  - Performance: Use `Some(1)` limit when only need latest sync time
- New endpoint pattern: `/ingestions/history/{history_id}` returns both config and history data
  - Uses `IngestionConfigHistoryView` for efficient joined queries
  - Single database call replaces multiple separate queries

<!-- AI-UPDATED: 2026-09-21 - Cross-tenant IDOR fix on GET /ingestions/history/{id} -->

- **Profile scoping lives in the query, not the handler**: `find_config_and_history_by_id_and_profile`
  filters on both `dsl::id` and `dsl::profile_id`
  - There is deliberately **no** unscoped `find_config_and_history_by_id` variant left — an unscoped
    point lookup on this view is never safe to hand back to a caller, and removing it makes omitting
    the profile a compile error instead of a cross-tenant disclosure
  - `JwtAuth::basic(Permission::…)` is class-level only; it never sees the `{id}`, so it cannot be
    relied on for object-level authorization
  - Wrong profile → same `IngestionHistoryNotFound` 404 as a missing row, never 403 (a 403 turns the
    endpoint into an oracle for enumerating other tenants' record IDs)
- API response evolution: `IngestionConfigResponse` structure changed significantly
  - Removed: `profile_id`, `account_id`, direct `config` field exposure
  - Added: `ingestion_id`, `last_synced_at`, typed `data` field with connection details
  - Breaking change: Clients expecting old response format will fail
- Connection data transformation pattern in `IngestionConfigDataApiExt`
  - Maps internal `IngestionConfigData` enum to API-friendly `IngestionConnectionData`
  - Handles sensitive credential exposure vs. API response requirements
  - Example: Adyen config exposes auth credentials in API response for client configuration

<!-- AI-UPDATED: 2025-09-10 - Ingestion-specific history query implementation -->

- **History queries are now ingestion-specific, not profile-level**:
  - `list_ingestion_configs` uses `list_ingestion_history_by_ingestion_id(&config.ingestion_id, Some(1))`
  - Each config gets its own `last_synced_at` from most recent history for that specific ingestion
  - Previous implementation used profile-level query that returned wrong timestamps
  - Pattern: Query history by `ingestion_id` with `discarded_at.is_null()` filter to exclude old versions

## File Path Handling

<!-- AI-UPDATED: 2025-07-24 - PathBuf refactoring for type safety -->

- File paths now use `PathBuf` instead of `String` throughout ingestion and transformation
  - **IngestionProcessedData.file_key**: Changed from `String` to `PathBuf`
  - **TransformationTriggerRequest.file_path**: Changed from `String` to `PathBuf`
  - **Critical**: Must convert PathBuf to &str for file storage operations using `.to_str()`
  - Returns `InvalidFileName` error if PathBuf contains non-UTF8 characters
- File key construction uses PathBuf's iterator pattern:
  ```rust
  ["ingestions", ingestion_id, history_id, file_name]
      .iter()
      .collect()
  ```
- **API Impact**: JSON serialization still uses String representation
  - OpenAPI schema annotated with `#[schema(value_type = String)]`
  - API consumers see no change in JSON structure
- **File name extraction**: Use `PathBuf::file_name()` for getting just the filename
  - Changed from storing full path to extracting filename component
  - Example: `data.file_key.file_name().and_then(|s| s.to_str())`

## SFTP Internal Implementation

<!-- AI-UPDATED: 2025-01-24 - SFTP Internal ingestion implementation -->

- **SFTP Internal** is a special ingestion type that retrieves files from S3 (not traditional SFTP)
  - Name kept as "SFTP Internal" for business reasons but uses S3 backend
  - Uses dedicated S3 bucket configured in `ingestion_service.sftp_file_storage`
  - Different from regular file storage to allow separate bucket/permissions
- **Configuration Structure**:
  - `SftpInternalIngestionConfig` only contains `file_path: PathBuf` - the S3 key
  - Storage config is in `IngestionState` via `ingestion_service.sftp_file_storage`
  - Allows different S3 buckets for different environments
- **IngestionState Pattern**:
  - All ingestion-specific resources grouped in `IngestionState` struct
  - Contains `config: IngestionServiceConfig` and `sftp_file_storage: FileStorageHandler`
  - Accessed via `state.ingestion.sftp_file_storage` instead of `state.file_storage`
- **Trigger Behavior**:
  - Can be triggered via webhook endpoint like other ingestion types
  - **Requires `x-internal-api-key`** (shared `auth.internal_api_key`). Intended caller: an internal cron job
  - Manual upload not supported - returns `ManualUploadNotSupported` error
  - Uses `TriggerDataV1::SftpInternal` and `RawPayloadV1::SftpInternal` variants
- **File Download Flow**:
  - `download_sftp_internal_file` takes `IngestionState` instead of full `AppState`
  - Retrieves file using `state.ingestion.sftp_file_storage.retrieve_file()`
  - Extracts filename from PathBuf for storage

## File Download API

<!-- AI-UPDATED: 2025-10-27 - Download ingested file endpoint implementation -->

- **Download Endpoint**: `GET /ingestions/history/{history_id}/download` returns processed file bytes
  - Must register route in `routes/app.rs` under `/history/{id}/download` scope
  - Easy to forget route registration - endpoint will return 404 instead of 401 without it
- **Response Type**: Uses `ApplicationResponse::FileDownload` variant (not JSON)
  - Contains: file bytes (`Vec<u8>`), filename (`String`), content_type (`mime::Mime`)
  - Handler in `services/api.rs` sets HTTP headers: `Content-Type` and `Content-Disposition: attachment`
  - Always uses `application/octet-stream` - frontend determines handling based on filename
- **Security**: Profile ownership verified before download
  - Returns 404 (not 403/401) if history belongs to different profile (prevents info leakage)
  - JWT authentication required - enforced at route level
- **File Availability**: Only successfully processed files can be downloaded
  - Pending/Failed status → 404 with `FileNotAvailable` error
  - Missing file in storage → 404 with `FileNotAvailable` error
  - Uses `IngestionDataExt` trait methods: `get_file_key()` and `get_file_name()`
- **Error Codes**: New error variants added to `IngestionError`
  - `IngestionHistoryNotFound` (IE-908) - History ID doesn't exist or wrong profile
  - `FileNotAvailable` (IE-909) - File not processed or missing from storage
- **Implementation Pattern**: Core function returns `(Vec<u8>, String)`, route handler wraps in `FileDownload`
  - Must import `IngestionDataExt` trait for file metadata extraction
  - PathBuf conversion to &str required for file storage retrieval

## Encryption Architecture

<!-- AI-UPDATED: 2025-08-03 - Added comprehensive encryption framework for sensitive config data -->

- **Encryption Flow**: API (`Secret<String>`) → Diesel (`Encryption`) → Domain (`EncryptableName`) → API (`EncryptableName`)
  - **Create/Update**: API `Secret<String>` → `EncryptionHandler` → Database `Encryption` storage
  - **Read**: Database `Encryption` → Domain decryption → `EncryptableName` → API response
  - Database stores encrypted data with versioned format: `v1:base64-encoded-data`
- **Affected API Endpoints**: All ingestion config CRUD operations now handle encryption
  - `POST /ingestions/config` - Encrypts sensitive fields during creation
  - `PUT /ingestions/config/{id}` - Re-encrypts modified sensitive data
  - `GET /ingestions/config` and `GET /ingestions/config/{id}` - Decrypt for responses
  - API requests use `Secret<String>`, responses return masked `EncryptableName`
- **New Types Introduced**:
  - `EncryptableName` - Type alias for `Encryptable<Secret<String>>` (prevents accidental exposure)
  - `CryptoOperation<T,S>` - Enum for encrypt/decrypt operations with batch support
  - `ApplicationEncryptionClient` - Main encryption client (Local or KeymanagerService)
  - `EncryptionHandler<T>` - Utility for streamlined encryption to database format
  - `Encryption` - Database storage type with versioned binary format
- **Encrypted Fields by PSP Type**:
  - **Adyen**: `hmac_secret`, `webhook_basic_auth_password`, `report_basic_auth_password`
  - **SftpInternal/Manual**: No encryption needed for current field set
- **Type Safety**: `EncryptableName` prevents accidental exposure of sensitive data in logs/responses
  - Cannot be accidentally printed or serialized without explicit conversion
  - Enforces encryption at compile time through type system
- **Breaking Changes in API Responses**:
  - Sensitive fields now return `EncryptableName` instead of plain strings (masked values)
  - New `[application_encryption]` config section required with encryption strategy
  - Database schema changes: sensitive fields stored as binary `Encryption` type
  - AppState now includes `encryption_client: ApplicationEncryptionClient`
- **Encryption Backend Flexibility**: Supports both KeymanagerService and LocalEncryption strategies
  - LocalEncryption: SHA256(master_key + merchant_id) for key derivation (AES-256-GCM)
  - KeymanagerService: External HTTPS service with certificate-based auth
  - Per-merchant key isolation using `Identifier::ReconMerchant(merchant_id)`
  - Configuration determines which encryption backend to use
- **Database Storage**: `Encryption` type with versioning support for future algorithm upgrades
  - Current version: `v1:` prefix with base64-encoded encrypted data
  - Diesel integration allows transparent database operations
  - Failed encryption operations surface as validation errors, not system panics

## Trigger Auth (`POST /ingestions/{id}/trigger`)

<!-- AI-UPDATED: 2026-10-05 - SFTP triggers require the internal API key -->

- **Route is `NoAuth`**: the config type is only known after the lookup, and Adyen cannot send `x-internal-api-key`. Do not switch the route to `InternalApiKeyAuth`; it would break Adyen
- Each ingestion context authenticates in `parse_payload`. There is no per-config match in core
- `SftpInternalIngestionContext` carries the request `headers` and the expected key (`Secret`, since the context derives `Debug`). `authenticate_sftp_internal` (`pub`) compares the `x-internal-api-key` header with it inline. The same comparison is repeated in `InternalApiKeyAuth::validate`, so keep the two in sync. Both reject an empty configured key
- A bad or missing key is `ParseStageError::InvalidAuthentication` → 400, same as Adyen. The `Pending → Failed` history row is written on purpose so the failure is visible to the client in history
- **Gotcha**: because auth runs after `insert_ingestion_history`, an anonymous caller who knows an `ingestion_id` can write `Failed` history rows for that config. A pre-insert authentication step would close this for SFTP and Adyen together
- `Manual` configs fail at context creation with `WebhookNotSupported`
- **Tests**: `.with_internal_key()` must come before `.json()` on the test `RequestBuilder`, which copies headers when `.json()` is called
