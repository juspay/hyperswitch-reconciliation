# Encryption - Agent Notes

## System Architecture

### Two-Strategy Design
- **LocalEncryption**: Uses SHA256(master_key + identifier) for key derivation
- **KeymanagerService**: Delegates all operations to external HTTPS keymanager service
- **No Fallback**: Each strategy is deterministic - LocalEncryption never touches keymanager, KeymanagerService never generates local keys

### Configuration Pattern
```toml
[application_encryption]
encryption_mode = "local_encryption"  # or "key_manager"

[application_encryption.local_encryption]
master_key = "base64-32-byte-key"

[application_encryption.key_manager]
enabled = true
url = "https://localhost:5000"
cert = "..."
ca = "..."
```

## Common Gotchas
<!-- AI-UPDATED: 2025-01-29 - Comprehensive encryption system implementation -->

### Configuration
- Master key MUST be exactly 32 bytes when base64 decoded - system validates this strictly
- Master key cannot be all zeros (weak key detection prevents this)
- KeyManager variant requires both `cert` and `ca` certificates for HTTPS communication
- Switching encryption_mode requires corresponding section configuration
- No separate [key_manager] section - everything is self-contained under application_encryption

### Secrets Management Integration
<!-- AI-UPDATED: 2025-08-04 - Critical SecretsHandler gotcha -->
- ApplicationEncryptionConfig MUST implement SecretsHandler trait to work with secrets management
- Missing SecretsHandler implementation causes compilation errors in fetch_raw_secrets()
- Both master_key (LocalEncryption) and cert/ca (KeyManager) fields are processed through KMS
- Values in config files are KMS ciphertext in production, plain text in development
- SecretsHandler implementation handles the SecuredSecret → RawSecret transition automatically

### Key Management
- LocalEncryption: Keys derived on-demand using SHA256(master_key + identifier), no storage needed
- KeymanagerService: Must call setup_key_for_identifier() before any encryption operation
- Per-identifier isolation: Different merchant IDs get completely different encryption keys
- Key creation in keymanager is idempotent - safe to call multiple times

### TypeEncryption Integration
- All TypeEncryption methods removed key parameters - keys come from encryption client
- Must call state.encryption_client.setup_key_for_identifier() before encryption operations
- CryptoOperation enum handles: Encrypt, EncryptOptional, Decrypt, DecryptOptional, BatchEncrypt, BatchDecrypt
- ApplicationEncryptionInterface matches crypto_operation signature exactly

### Error Handling
- LocalEncryption errors: InvalidConfiguration, EncryptionFailed, DecryptionFailed
- KeymanagerService errors: Include network failures, certificate issues, service unavailability
- No recovery mechanisms - if configured strategy fails, operation fails
- Validation happens at startup - bad configuration prevents application start

## Design Decisions
<!-- AI-UPDATED: 2025-01-29 - Discovered during implementation -->

### Self-Contained Variants
- Each encryption variant contains ALL its configuration (mirrors secrets_management pattern)
- No shared state between LocalEncryption and KeymanagerService
- Configuration switching is clean - change mode and corresponding subsection

### Interface Design
- ApplicationEncryptionInterface trait abstracts both strategies
- setup_key_for_identifier() is strategy-specific: no-op for Local, API call for Keymanager
- crypto_operation() matches existing crypto module signature for easy migration

### Security Architecture
- AES-256-GCM for all encryption operations
- LocalEncryption: SHA256 key derivation ensures cryptographic isolation per identifier
- KeymanagerService: HTTPS with client certificate authentication
- All sensitive data uses Secret<T> type with proper masking

## Integration Points
<!-- AI-UPDATED: 2025-01-29 - Key integration discoveries -->

### Application State
- encryption_client: ApplicationEncryptionClient stored in AppState for global access
- Initialized once during application startup after config validation
- Available to all routes and core business logic

### Merchant Account Integration
- Used for encrypting sensitive merchant data during account creation
- Identifier pattern: Identifier::ReconMerchant(merchant_id)
- Always call setup_key_for_identifier() before crypto operations in merchant workflows
<!-- AI-UPDATED: 2025-08-05 - Business profile encryption integration -->
- Business profile names are encrypted using merchant-specific keys via `get_encryptor_for_merchant(merchant_id)`
- Domain model conversion handles automatic decryption via `BusinessProfile::convert_back()` using merchant encryptor

### Configuration Loading
- ApplicationEncryptionConfig integrated into main Config<S> struct
- Validation integrated into config.validate() pipeline
- Follows same secret transformation pattern as other secure configs

### Secrets Management Integration
<!-- AI-UPDATED: 2025-08-04 - SecretsHandler implementation -->
- ApplicationEncryptionConfig implements SecretsHandler trait for KMS encryption support
- Master keys and certificates are now encrypted at rest using KMS in production
- Works seamlessly with both no_encryption (dev) and aws_kms (prod) modes
- Processed through fetch_raw_secrets() pipeline alongside DatabaseConfig and AuthConfig
- Configuration values transition from SecuredSecret to RawSecret state during startup

## Critical Rules
<!-- AI-UPDATED: 2025-01-29 - Must-follow patterns -->

- **Strategy Determinism**: Never implement fallback between LocalEncryption and KeymanagerService
- **Key Lifecycle**: For KeymanagerService, ALWAYS call setup_key_for_identifier() before encryption
- **Identifier Consistency**: Use same identifier format (Identifier::ReconMerchant) across all encryption calls
- **Configuration Validation**: ApplicationEncryptionConfig.validate() must be called during startup
- **Error Propagation**: Let encryption errors bubble up - don't mask or retry automatically

## Performance Characteristics
<!-- AI-UPDATED: 2025-01-29 - Operational insights -->

### LocalEncryption
- Key derivation: Fast SHA256 computation per operation
- No network calls: Suitable for high-throughput scenarios
- Memory usage: Minimal - keys derived on demand, not stored

### KeymanagerService
- Network dependent: Each operation requires HTTPS round-trip to keymanager
- Key creation: One-time cost per identifier
- Certificate validation: Startup cost, then cached connections
- Timeout considerations: client_idle_timeout configurable for connection pooling

## Future Extensibility
<!-- AI-UPDATED: 2025-01-29 - Architecture flexibility -->

### Adding New Strategies
- Extend ApplicationEncryptionConfig enum with new variant
- Implement ApplicationEncryptionInterface for new strategy
- Add to ApplicationEncryptionClient enum
- Update get_encryption_client() factory method
- Add configuration validation for new strategy

### Potential Strategies
- AWS KMS integration for cloud-native deployments
- HashiCorp Vault for enterprise secret management
- Hardware Security Module (HSM) integration
- Multi-region key management

## Testing Strategies
<!-- AI-UPDATED: 2025-01-29 - Testing approach -->

### Unit Testing
- Test each strategy in isolation with mock configurations
- Validate key derivation determinism for LocalEncryption
- Mock keymanager API responses for KeymanagerService testing

### Integration Testing
- Test application startup with both encryption strategies
- Validate end-to-end encryption/decryption workflows
- Test configuration switching between strategies

### Security Testing
- Verify key isolation between different identifiers
- Confirm encrypted data is not readable without proper keys
- Validate certificate authentication for keymanager communication

## When Working Here
<!-- AI-UPDATED: 2025-01-29 - Developer guidance -->

- Read external_services/encryption/config.rs for main interfaces
- Check existing implementations in external_services/encryption/implementations/
- Update merchant account creation code when adding new encrypted fields
- Test both encryption strategies when modifying encryption logic
- Validate configuration changes with both local and keymanager modes
<!-- AI-UPDATED: 2025-08-05 - Domain model patterns -->
- When adding encryption to new fields, implement `Conversion` trait for automatic encrypt/decrypt in domain models
- Use `CryptoOperation` enum with appropriate merchant identifier for consistent encryption patterns
- Always get merchant-specific encryptor before crypto operations: `state.encryption_client.get_encryptor_for_merchant()`

## Staging Entries Metadata Encryption

<!-- AI-UPDATED: 2025-08-13 - Comprehensive staging entries encryption implementation -->
### Implementation Overview
- **Migration**: Database schema updated with merchant_id column and metadata type change from JSONB to BYTEA
- **Encryption Strategy**: All staging entry metadata encrypted using merchant-specific keys
- **Key Isolation**: Each merchant gets separate encryption keys via `Identifier::ReconMerchant(merchant_id)`
- **Storage Format**: Consistent `v1:base64` format across both LocalEncryption and KeymanagerService strategies

### Batch Processing Optimization
- **Problem**: Individual row-by-row encryption created O(n) performance bottleneck for large CSV files
- **Solution**: 3-pass batch processing pattern eliminates individual encryption calls
- **Performance**: Achieved 10-40x improvement for files with 10,000+ entries
- **Implementation**: Applied across all transformation functions

<!-- AI-UPDATED: 2026-05-21 - Batch chunking, resource-scoped constants, transaction wrap -->
<!-- AI-UPDATED: 2026-06-07 - Batch decrypt now chunked too -->
### Batch chunking
- Encrypt: `batch_encrypt_chunked_via_api` delegates to a shared generic `chunked_batch_encrypt<Out, V, S>` free function inside `mod encrypt` (KeyManager body limit returns HTTP 413 above ~8k entries). Decrypt is chunked the same way: `batch_decrypt_chunked_via_api` delegates to the sibling `chunked_batch_decrypt<Out>`. Local key-based `batch_encrypt`/`batch_decrypt` aren't chunked — in-memory, no payload limit.
- DB insert: `StagingEntryNew::batch_insert_chunked` chunks via the `common_utils::chunking::chunked` helper, wrapped in `transaction_async` so all chunks commit or none do (PG bind-param cap is 65,535).
- The generic `chunked()` helper lives in `common_utils/src/chunking.rs`. Numeric policy constants are **resource-scoped**: `BATCH_ENCRYPTION_CHUNK_SIZE` / `BATCH_ENCRYPTION_MAX_CONCURRENT` (and `BATCH_DECRYPTION_*` counterparts) in `recon_domain_models/src/type_encryption.rs`, `STAGING_ENTRY_BATCH_CHUNK_SIZE` in `diesel_models/src/query/staging_entry.rs`. Each is tuned against a different downstream limit and should be retuned independently.
- 413 surfaces as `KeyManagerClientError::PayloadTooLarge` (mapped in `keymanager.rs::call_encryption_service`)

### Critical Patterns
- **Merchant Validation**: Batch operations validate all items belong to same merchant for security
- **Index-Based Lookup**: Use proper index mapping instead of fragile key-value pairing approaches
- **Error Propagation**: Encryption failures properly bubble up as domain-specific errors
- **Function Signatures**: All transformation functions require `encryption_client` parameter for merchant-specific encryption

### Database Schema Changes
```sql
-- Add merchant_id column for encryption key isolation
ALTER TABLE staging_entries ADD COLUMN merchant_id VARCHAR(64) NOT NULL;

-- Change metadata from JSONB to BYTEA for encrypted storage
ALTER TABLE staging_entries ALTER COLUMN metadata TYPE BYTEA 
USING convert_to(metadata::text, 'UTF8');
```

### Gotchas and Security Considerations
- **Never mix merchants**: Batch encryption/decryption operations must validate merchant_id consistency
- **Index mapping required**: Use `(idx.to_string(), metadata)` pattern for FxHashMap keys in batch operations
- **Encryption client lifecycle**: Pass encryption_client through entire transformation pipeline, not individual encryptors
- **Type consistency**: Local and KeyManager encryption must produce same `v1:base64` format for data portability

## Entries Metadata Encryption

<!-- AI-UPDATED: 2025-08-13 - Complete entries metadata encryption implementation -->
### Implementation Overview
- **Database Migration**: Added merchant_id column and changed metadata from JSONB to BYTEA in entries table
- **Architecture Pattern**: 3-layer design exactly mirroring staging_entries implementation
  - API Models: Simple structure with SecretValue (requests) and EncryptableValue (responses)
  - Diesel Models: Complex structure with Encryption type for encrypted database storage
  - Domain Models: Complex structure with EncryptableValue for business logic
- **Conversion Patterns**: Domain models implement `Conversion` trait for automatic encrypt/decrypt operations

### Critical Implementation Patterns
  - Implementation hashes the exposed inner value for deterministic behavior
  - Uses uuid-based mapping with `FxHashMap<String, Encryption>` for batch decryption
- **Architecture Boundary Enforcement**: Clear separation between core and utils layers
  - Core layer: Handles encryption/decryption and passes encryptor to utils functions
  - Utils layer: Receives encryption context and performs metadata encryption during entry creation

### Reconciliation Engine Integration
- **Field Access Pattern**: ProcessAsConfirmation converts Diesel entries to Domain entries for metadata field access
  - Uses `DomainEntry::convert_back()` to decrypt expected entry before field extraction
  - Utils functions like `extract_field_value_from_entry` work with Domain models, not Diesel models
- **Async Function Signatures**: Utils layer functions made async to support encryption operations
  - Transaction creation functions require merchant_id and encryptor parameters
  - Encryption context propagated through entire call chain from core to utils layers

### Security and Performance
- **Merchant Isolation**: All entries use merchant-specific encryption keys via `Identifier::ReconMerchant(merchant_id)`
- **Batch Validation**: Batch operations validate merchant_id consistency across all items before processing
- **Error Handling**: Encryption failures bubble up as domain-specific ValidationError types
