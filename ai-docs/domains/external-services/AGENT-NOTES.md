# External Services - Agent Notes

## Critical Rules
<!-- AI-UPDATED: 2025-03-13 - User client architecture refactor -->
- **User Client Abstraction**: All user-related operations go through `UserClient` enum (`user_client`), not directly through Hyperswitch
- **Two Backend Modes**: `MockUserClient` (`mock_user_client`) for local/mock data, and `Hyperswitch` (`hyperswitch_client`) for actual HTTP calls to HyperSwitch
- **Interface Pattern**: `UserInterface` trait provides common operations across both backends
- **Production vs Local**: Production uses `client_mode = "hyperswitch"`, local/testing uses `client_mode = "mock_user_client"`
<!-- AI-UPDATED: 2025-01-07 - Secret management implementation -->
- All secrets MUST use `Secret<T>` wrapper for type safety
- Use `.peek()` to access secret values, never direct field access
- Secret transformers must wrap results in `Secret::new()`
- All config secrets (jwt_secret, admin_api_key, internal_api_key, db_password) use Secret<T>
- **Gotcha**: two unrelated keys are both called `internal_api_key`: `auth.internal_api_key` (recon's own service-to-service key, checked on incoming requests, KMS-decrypted in prod, must be non-empty) and `hyperswitch_client.internal_api_key` (sent to Hyperswitch). Do not mix them up
<!-- AI-UPDATED: 2025-01-07 - File storage security -->
- File keys MUST be validated before use - no empty keys, no directory traversal
- S3 file keys cannot start with '/' or contain null characters
- FileSystem storage automatically prevents directory traversal attacks
- Always validate file storage config before creating clients

## Common Gotchas
<!-- AI-UPDATED: 2026-08-20 - FaaS invocation backend added -->
- **FaaS defaults to `Noop`**: a missing `[faas]` config section deserializes to `Noop`, which logs and returns `Ok` - invocations vanish silently instead of failing loudly
- **The abstraction is `faas`, the backend is `aws_lambda`**: vendor names belong in the variant only, same as `file_storage_backend` -> `AwsS3`; `FaasInterface::invoke_event` is the single operation
- **FaaS invocations are fire-and-forget**: `InvocationType::Event` returns 202 once queued and `function_error` is always `None`, so success means "accepted", never "the report was produced"
- **`AwsLambdaConfig` has no endpoint field on purpose**: `aws_config::defaults().load()` already reads `AWS_ENDPOINT_URL`, so retargeting the backend is an env var, not a config change
- **Report generation is `recon`-crate business logic, not part of `faas`**: `state.faas.invoke_event(function_name, payload)` is the only thing report generation uses from this crate - the report-type-to-function-name mapping, request validation, and payload shape all live in `crates/recon/src/{core,domain,errors}/reporting.rs`, see `ai-docs/domains/reporting/AGENT-NOTES.md`
<!-- AI-UPDATED: 2025-03-13 - User client gotchas from refactor -->
- **Config Breaking Change**: `[hyperswitch_client]` config section removed, replaced with `[user_client]`
- **Mock User Response**: MockUserClient creates predictable test users with pattern: name="User {user_id}", email="{user_id}@test.com"
- **UserResponse Rename**: `HyperSwitchUserResponse` and `HyperSwitchBatchUser` merged into unified `UserResponse`
- **AppState Field Rename**: `hyperswitch_client` field renamed to `user_client` in AppState
- **SecretsHandler Chain**: UserConfig implements SecretsHandler which delegates to HyperswitchConfig for encryption
- **Header Construction**: AuthHeaders trait now returns `CustomResult<Vec<(HeaderName, HeaderValue)>>` not simple Vec
<!-- AI-UPDATED: 2025-01-07 - Secret<T> wrapper gotchas -->
- Debug output shows `*** String ***` instead of actual secret values
- Secrets are serde transparent - deserialize directly to inner type
- Must use `.peek()` for read access - no direct field access allowed
- Secret transformers receive raw values but must return wrapped secrets
- Config access patterns changed: `config.jwt_secret.peek()` not `config.jwt_secret`
<!-- AI-UPDATED: 2025-01-07 - File storage gotchas -->
- AWS S3 presigned URLs only work with S3 backend, returns None for FileSystem
- FileSystem storage creates parent directories automatically if `create_directories: true`
- S3 file_exists() uses HEAD request - can be expensive for frequent checks
- File keys are case-sensitive in both S3 and FileSystem backends
- S3 endpoint_url is optional - only needed for S3-compatible services

## Design Decisions
<!-- AI-UPDATED: 2026-08-20 - Lambda client construction -->
- **Lambda SDK client is built once in `AwsLambdaClient::new()`**, never per invocation
  - Rebuilding re-runs the entire credential provider chain on every call
  - Mirrors `AwsS3Storage::new()`; hyperswitch's `get_aws_client()` rebuilds per call - do not copy that
<!-- AI-UPDATED: 2025-03-13 - User client abstraction architecture -->
- **User Client Abstraction Pattern**: Created unified `UserClient` enum wrapping multiple backends
  - `MockUserClient`: Local testing/development with predictable mock data
  - `HyperswitchClient`: Production HTTP client calling HyperSwitch APIs
  - Unified `UserInterface` trait for common operations (get_user_by_id, list_users_by_ids)
- **Configuration Tag Pattern**: `client_mode` field selects backend at runtime
  - `mock_user_client`: Zero-config local development
  - `hyperswitch`: Production with nested `hyperswitch_config` section
- **Type Unification**: Single `UserResponse` type instead of separate response types per backend
<!-- AI-UPDATED: 2026-03-31 - list_assignees endpoint and User model rename -->
- **ForeignFrom Conversion**: `UserResponse` → `api_models::User` conversion centralized via `ForeignFrom` impl in `recon/utils/user.rs`
- **List Assignees Endpoint**: `GET /exception_management/assignees/list` returns profile-scoped assignable users via HyperSwitch `/user/internal/members?access_level=profile`
- **SecretsHandler Delegation**: UserConfig implements SecretsHandler, delegates to HyperswitchConfig for actual secret decryption
- **Reqwest Client Reuse**: Uses OnceCell for singleton HTTP client with connection pooling
<!-- AI-UPDATED: 2025-01-07 - Secret management architecture -->
- Secret<T> uses strategy pattern for masking in debug output
- WithType strategy shows type name instead of value when debugging
- Secrets are Clone but maintain wrapper safety
- Serde transparency allows seamless deserialization from config files
- PhantomData used for zero-cost masking strategy
<!-- AI-UPDATED: 2025-01-07 - File storage architecture -->
- FileStorageInterface trait provides common operations across backends
- Tagged enum config pattern allows runtime backend selection
- Error types are specific to operation type (UploadFailed, RetrieveFailed, etc.)
- Async trait pattern used for all file operations
- Default backend is FileSystem with "./storage" base path

## User Client Architecture
<!-- AI-UPDATED: 2025-03-13 - User client system refactor -->
The user client system provides three related concepts: `user_client` (the abstraction), `mock_user_client` (local backend), and `hyperswitch_client` (production backend).

- **Module Structure**: 
  - `external_services::user`: Main module with interface, types, config, implementations
  - `external_services::hyperswitch`: Now only contains HTTP client implementation
- **UserClient Enum** (`user_client`): 
  - `MockUserClient` (`mock_user_client`): Local service returning mock data
  - `Hyperswitch` (`hyperswitch_client`): Production HTTP client
- **UserInterface Trait**: Async trait providing `get_user_by_id()`, `list_users_by_ids()`, and `list_assignees()`
- **Unified Response**: `UserResponse` struct used by both backends (user_id, name, email, is_active)
- **Mock Implementation** (`mock_user_client`): Returns predictable test users without external calls
- **HyperSwitch Implementation** (`hyperswitch_client`): Makes authenticated HTTP calls with internal API key
- **Config Pattern**:
  ```toml
  [user_client]
  client_mode = "mock_user_client"  # or "hyperswitch"
  
  [user_client.hyperswitch_config]  # only needed for hyperswitch mode
  base_url = "..."
  internal_api_key = "..."
  request_timeout_secs = 30
  client_idle_timeout = 90
  ```

## Secret Management Architecture
<!-- AI-UPDATED: 2025-01-07 - Secrets system design -->
- **SecretStateContainer<T, S>**: Type-safe state transitions between SecuredSecret and RawSecret
- **SecretManagementInterface**: Common trait for AWS KMS and NoEncryption backends
- **SecretsHandler**: Trait for converting secured secrets to raw secrets
- **State transitions**: SecuredSecret -> RawSecret via `transition_state()` method
- **Configuration**: Tagged enum with `secrets_manager` field selects backend

## Secret Management Backends
<!-- AI-UPDATED: 2025-01-07 - Backend implementations -->
- **AWS KMS**: Encrypts/decrypts using KMS keys, requires key_id and region
- **NoEncryption**: Development/testing backend, supports environment variable substitution
- **Environment variables**: NoEncryption handles `${VAR_NAME}` syntax automatically
- **Base64 encoding**: AWS KMS expects base64-encoded ciphertext input
- **Validation**: Both backends validate configuration before client creation

## File Storage Patterns
<!-- AI-UPDATED: 2025-01-07 - Implementation patterns -->
- `FileStorageInterface::upload_file(key, data)` for storing files
- `FileStorageInterface::retrieve_file(key)` returns Vec<u8> data
- `FileStorageInterface::delete_file(key)` removes files
- `FileStorageInterface::file_exists(key)` checks existence
- `FileStorageInterface::generate_presigned_url(key, expires)` for S3 only

## File Storage Backends
<!-- AI-UPDATED: 2025-01-07 - Backend specifics -->
- **AWS S3**: Full S3 integration with bucket, region, optional endpoint_url
- **FileSystem**: Local storage with configurable base_path and directory creation
- **S3 features**: Presigned URLs, path-style addressing, custom endpoints
- **FileSystem security**: Canonical path validation prevents directory traversal
- **Configuration**: Tagged enum with `file_storage_backend` field

## When Working Here
<!-- AI-UPDATED: 2025-03-13 - User client development -->
- Use `UserInterface` trait for user operations, not concrete types
- Import from `external_services::user` not `external_services::hyperswitch` directly
- AppState contains `user_client: UserClient` field (not `hyperswitch_client`)
- MockUserClient requires no external configuration - perfect for tests
- UserResponse struct provides all user data needed for audit trails
- When adding new backends: implement UserInterface + update UserConfig + update UserClient enum
<!-- AI-UPDATED: 2025-01-07 - Development gotchas -->
- Always import `Secret` from `external_services::secrets`
- Check if features are enabled in Cargo.toml (serde feature)
- Secret transformers in `secrets_transformers.rs` handle config loading
- Use `.peek().clone()` when passing secrets to external APIs
- Never log or print secrets directly - wrapper prevents this
<!-- AI-UPDATED: 2025-01-07 - File storage development -->
- Import file storage types from `external_services::file_storage`
- Validate file keys before operations to get clear error messages
- Use `file_exists()` before `retrieve_file()` to avoid FileNotFound errors
- S3 operations require AWS credentials in environment or IAM roles
- FileSystem operations require write permissions to base_path directory

## Configuration Integration
<!-- AI-UPDATED: 2025-03-13 - User client configuration -->
- **New Config Format** (replaces old `[hyperswitch_client]` section):
  ```toml
  [user_client]
  client_mode = "mock_user_client"  # or "hyperswitch"
  
  # Only for hyperswitch mode:
  [user_client.hyperswitch_config]
  base_url = "http://localhost:8080"
  internal_api_key = "test_internal_api_key"
  request_timeout_secs = 30
  client_idle_timeout = 90
  ```
- **Migration from old format**: Old `[hyperswitch_client]` config removed entirely
- **SecretsHandler chain**: UserConfig → delegates to HyperswitchConfig for decryption
- **Environment-specific defaults**:
  - `config.toml`: `mock_user_client` (local development)
  - `test.toml`: `mock_user_client` (testing)
  - `ci.toml`: `mock_user_client` (CI pipelines)
  - `prod.toml`: `hyperswitch` (production with env var substitution)
<!-- AI-UPDATED: 2025-01-07 - Config system integration -->
- Config deserialization automatically wraps secrets
- Secret transformers fetch from secret management and re-wrap
- Database URL construction uses `.peek()` for password access
- JWT encoding/decoding uses `.peek()` for secret access
- Admin API key validation uses `.peek()` for comparison
<!-- AI-UPDATED: 2025-01-07 - File storage config -->
- File storage config uses tagged enum pattern for backend selection
- Default config creates FileSystem backend with "./storage" path
- S3 config requires bucket_name and region, validates on creation
- FileSystem config validates base_path exists or can be created

## Error Handling Patterns
<!-- AI-UPDATED: 2025-01-07 - Error management -->
- **Secrets**: SecretsManagementError with FetchSecretFailed, ClientCreationFailed
- **File Storage**: FileStorageError with operation-specific variants
- **AWS KMS**: Detailed errors for base64, encryption, decryption failures
- **Context-rich errors**: All errors include file paths, operations, and backend info
- **Error propagation**: Uses error-stack for detailed error context

## Testing Considerations
<!-- AI-UPDATED: 2025-01-07 - Testing with secrets -->
- Test configs use hardcoded secret values wrapped in Secret<T>
- Mock secret management returns wrapped values
- Integration tests must handle Secret<T> in assertions
- Use `.peek()` when comparing secret values in tests
<!-- AI-UPDATED: 2025-01-07 - File storage testing -->
- FileSystem tests use temporary directories for isolation
- S3 tests require AWS credentials or mocking
- Test file key validation separately from backend operations
- Use small test files to avoid memory issues in tests

## Security Benefits
<!-- AI-UPDATED: 2025-01-07 - Security improvements -->
- Prevents accidental logging of sensitive values
- Type system enforces conscious access to secrets
- Debug output is safe for logs and error messages
- Compile-time prevention of secret leakage
- Clear distinction between raw and protected values
<!-- AI-UPDATED: 2025-01-07 - File storage security -->
- File key validation prevents directory traversal attacks
- Canonical path checking ensures files stay within base directory
- S3 file keys validated for AWS compatibility
- No execution of uploaded files - pure storage operations

## AWS Integration Gotchas
<!-- AI-UPDATED: 2025-01-07 - AWS-specific issues -->
- **KMS**: Requires base64-encoded input, returns UTF-8 decoded output
- **S3**: Object keys are case-sensitive and globally unique within bucket
- **Regions**: Both KMS and S3 require valid AWS region configuration
- **Credentials**: Uses default AWS credential chain (env vars, IAM roles, profiles)
- **Endpoints**: S3 supports custom endpoints for S3-compatible services
- **Path style**: S3 can use path-style or virtual-hosted-style addressing
<!-- AI-UPDATED: 2026-08-19 - SDK error opacity -->
- **`change_context` on an `SdkError` prints only "service error"**: the real cause sits in the source chain, so wrap in `DisplayErrorContext` before attaching or you lose the 409/auth/connection detail

## Application Encryption Architecture
<!-- AI-UPDATED: 2025-01-29 - Encryption system integration -->
- **ApplicationEncryptionInterface**: Common trait for LocalEncryption and KeymanagerService
- **Strategy Pattern**: Two self-contained encryption strategies with no fallback
- **Configuration**: Tagged enum with `encryption_mode` field selects strategy
- **Integration**: ApplicationEncryptionClient stored in AppState for global access
- **Validation**: Comprehensive config validation with security checks

## Encryption Strategies
<!-- AI-UPDATED: 2025-01-29 - Strategy implementations -->
- **LocalEncryption**: SHA256 key derivation, AES-256-GCM, self-contained
- **KeymanagerService**: External HTTPS service, certificate authentication
- **Key Management**: setup_key_for_identifier() before operations for keymanager
- **Deterministic**: No fallback between strategies - clean failure modes

## Encryption Integration Gotchas
<!-- AI-UPDATED: 2025-01-29 - Implementation discoveries -->
- ApplicationEncryptionConfig follows secrets_management pattern exactly
- Master key must be exactly 32 bytes when base64 decoded
- KeyManager variant requires both cert and ca for HTTPS communication
- No separate [key_manager] section - self-contained under application_encryption
- TypeEncryption methods removed key parameters - use encryption client
- Must call setup_key_for_identifier() before encryption for keymanager variant

## Development vs Production
<!-- AI-UPDATED: 2025-03-13 - User client environment setup -->
- **User Client Development**: 
  - Local: `client_mode = "mock_user_client"` - returns test users without network calls
  - Production: `client_mode = "hyperswitch"` - makes actual HTTP calls to HyperSwitch
  - Testing: Always use mock_user_client for deterministic, fast tests
- **Environment-specific configs**:
  ```toml
  # config.toml (local development)
  [user_client]
  client_mode = "mock_user_client"
  
  # prod.toml (production)
  [user_client]
  client_mode = "hyperswitch"
  
  [user_client.hyperswitch_config]
  base_url = "${HYPERSWITCH_CLIENT_BASE_URL}"
  internal_api_key = "${HYPERSWITCH_CLIENT_INTERNAL_API_KEY}"
  ```
<!-- AI-UPDATED: 2025-01-07 - Environment differences -->
- **Development**: Use NoEncryption for secrets, FileSystem for storage, LocalEncryption for data
- **Production**: Use AWS KMS for secrets, S3 for storage, KeymanagerService for data
- **Environment variables**: NoEncryption supports `${VAR}` substitution
- **Local testing**: FileSystem creates "./storage" directory automatically
- **AWS testing**: Requires valid AWS credentials and permissions
- **Configuration switching**: Change `secrets_manager`, `file_storage_backend`, and `encryption_mode` tags
