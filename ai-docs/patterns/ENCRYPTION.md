# Encryption Implementation Patterns

## Adding Encryption to New Data Types

### 1. Define TypeEncryption Implementation
```rust
use recon_domain_models::type_encryption::{TypeEncryption, CryptoOperation, CryptoOutput};
use common_utils::crypto;
use masking::{Secret, Strategy};

impl<S: Strategy<MyDataType>> TypeEncryption<MyDataType, crypto::GcmAes256, S> 
for crypto::Encryptable<Secret<MyDataType, S>>
{
    async fn encrypt_via_api(
        state: &AppState,
        data: Secret<MyDataType, S>,
        identifier: Identifier,
    ) -> CustomResult<Secret<String>, ApplicationEncryptionError> {
        // 1. Ensure key exists (required for keymanager)
        state.encryption_client
            .setup_key_for_identifier(&identifier)
            .await?;
        
        // 2. Perform encryption operation
        let result = state.encryption_client
            .crypto_operation(CryptoOperation::Encrypt(data), identifier)
            .await?;
            
        match result {
            CryptoOutput::Operation(encrypted) => Ok(encrypted),
            _ => Err(ApplicationEncryptionError::EncryptionFailed),
        }
    }
    
    async fn decrypt_via_api(
        state: &AppState,
        data: Secret<String>,
        identifier: Identifier,
    ) -> CustomResult<Secret<MyDataType, S>, ApplicationEncryptionError> {
        let result = state.encryption_client
            .crypto_operation(CryptoOperation::Decrypt(data), identifier)
            .await?;
            
        match result {
            CryptoOutput::Operation(decrypted) => Ok(decrypted),
            _ => Err(ApplicationEncryptionError::DecryptionFailed),
        }
    }
}
```

### 2. Integration in Business Logic
```rust
// In your service/core logic
async fn create_entity_with_encryption(
    state: &AppState,
    entity_data: CreateEntityRequest,
) -> Result<EntityResponse, ServiceError> {
    let identifier = Identifier::ReconMerchant(entity_data.merchant_id.clone());
    
    // 1. Create the entity object
    let entity = Entity {
        id: generate_entity_id(),
        merchant_id: entity_data.merchant_id,
        // ... other fields
    };
    
    // 2. Encrypt sensitive fields BEFORE database insertion
    let encrypted_data = crypto::Encryptable::encrypt_via_api(
        state,
        Secret::new(entity_data.sensitive_field),
        identifier,
    ).await?;
    
    // 3. Store in database
    entity.insert(&state.store.master_pool).await?;
    
    Ok(EntityResponse::from(entity))
}
```

## Identifier Patterns

### Merchant-Scoped Encryption
```rust
// For merchant-specific data
let identifier = Identifier::ReconMerchant(merchant_id.clone());
```

### Future: Multi-Level Identifiers
```rust
// When extending Identifier enum
pub enum Identifier {
    ReconMerchant(String),
    // Future extensions:
    // ReconUser(String),
    // ReconTransaction(String),
    // Global, // For system-wide encryption
}
```

## Configuration Patterns

### Local Development Configuration
```toml
[application_encryption]
encryption_mode = "local_encryption"

[application_encryption.local_encryption]
master_key = "generated_with_openssl_rand_base64_32"
```

### Production Configuration
```toml
[application_encryption]
encryption_mode = "key_manager"

[application_encryption.key_manager]
enabled = true
url = "https://keymanager.production.com"
cert = "-----BEGIN CERTIFICATE-----..."
ca = "-----BEGIN CERTIFICATE-----..."
client_idle_timeout = 30000
```

### Configuration Generation
```bash
# Generate master key for local encryption
openssl rand -base64 32

# Validate base64 key length
echo "your_key_here" | base64 -d | wc -c  # Should output 32
```

## Error Handling Patterns

### Service Layer Error Conversion
```rust
use external_services::encryption::ApplicationEncryptionError;

impl From<ApplicationEncryptionError> for ServiceError {
    fn from(err: ApplicationEncryptionError) -> Self {
        match err {
            ApplicationEncryptionError::EncryptionFailed => {
                ServiceError::InternalServerError
            },
            ApplicationEncryptionError::DecryptionFailed => {
                ServiceError::DataCorruption
            },
            ApplicationEncryptionError::InvalidConfiguration { message } => {
                ServiceError::ConfigurationError(message)
            },
        }
    }
}
```

### Graceful Error Messages
```rust
// In your handler/controller
async fn create_merchant_handler(
    state: AppState,
    request: CreateMerchantRequest,
) -> Result<Response, Error> {
    match create_merchant_with_encryption(&state, request).await {
        Ok(merchant) => Ok(Response::Created(merchant)),
        Err(ServiceError::ConfigurationError(msg)) => {
            // Log internal details
            tracing::error!("Encryption configuration error: {}", msg);
            // Return user-friendly message
            Err(Error::InternalServerError("Service temporarily unavailable"))
        },
        Err(other) => Err(other.into()),
    }
}
```

## Testing Patterns

### Unit Testing with Mocked Encryption
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use external_services::encryption::ApplicationEncryptionInterface;
    
    struct MockEncryptionClient;
    
    #[async_trait]
    impl ApplicationEncryptionInterface for MockEncryptionClient {
        async fn setup_key_for_identifier(&self, _: &Identifier) 
            -> CustomResult<(), ApplicationEncryptionError> {
            Ok(())
        }
        
        async fn crypto_operation<T, S>(&self, operation: CryptoOperation<T, S>, _: Identifier) 
            -> CustomResult<CryptoOutput<T, S>, ApplicationEncryptionError> {
            match operation {
                CryptoOperation::Encrypt(data) => {
                    // Mock encryption: just wrap in base64
                    let encrypted = base64::encode(format!("encrypted_{:?}", data));
                    Ok(CryptoOutput::Operation(Secret::new(encrypted)))
                },
                // ... other operations
            }
        }
    }
    
    #[tokio::test]
    async fn test_encryption_integration() {
        let state = create_test_app_state_with_encryption(MockEncryptionClient).await;
        // ... test logic
    }
}
```

### Integration Testing with Real Encryption
```rust
#[tokio::test]
async fn test_end_to_end_encryption() {
    let config = TestConfig {
        application_encryption: ApplicationEncryptionConfig::LocalEncryption {
            local_encryption: LocalEncryptionConfig {
                master_key: Secret::new("test_key_base64_encoded_32_bytes_long".to_string()),
            },
        },
    };
    
    let state = create_test_app_state(config).await;
    
    // Test encryption/decryption round trip
    let original_data = "sensitive_merchant_data";
    let identifier = Identifier::ReconMerchant("test_merchant_123".to_string());
    
    let encrypted = state.encryption_client
        .crypto_operation(
            CryptoOperation::Encrypt(Secret::new(original_data.to_string())), 
            identifier.clone()
        )
        .await
        .unwrap();
    
    let decrypted = state.encryption_client
        .crypto_operation(
            CryptoOperation::Decrypt(encrypted), 
            identifier
        )
        .await
        .unwrap();
    
    assert_eq!(decrypted.peek(), original_data);
}
```

## Performance Considerations

### LocalEncryption Strategy
```rust
// High-throughput scenarios - no network calls
// Key derivation cost: ~1μs per operation
// Suitable for: Development, testing, simple deployments
```

### KeymanagerService Strategy
```rust
// Network-dependent performance
// Key creation: One-time cost per identifier (~100ms)
// Encryption: Network round-trip per operation (~10-50ms)
// Suitable for: Production, compliance requirements
```

### Optimization Patterns
```rust
// Batch operations when possible
let batch_operations = vec![
    CryptoOperation::Encrypt(data1),
    CryptoOperation::Encrypt(data2),
    // ...
];

// Some implementations may support batch processing
let results = state.encryption_client
    .crypto_operation(CryptoOperation::BatchEncrypt(batch_operations), identifier)
    .await?;
```

## Security Best Practices

### Key Isolation
```rust
// ✅ Good: Different identifiers = different keys
let merchant_a_id = Identifier::ReconMerchant("merchant_a".to_string());
let merchant_b_id = Identifier::ReconMerchant("merchant_b".to_string());
// These will have completely different encryption keys

// ❌ Bad: Reusing identifiers across different data types
// Don't use the same identifier for different types of sensitive data
```

### Secret Handling
```rust
// ✅ Good: Use Secret<T> wrapper
let sensitive_data = Secret::new(user_input);

// ❌ Bad: Raw string handling
let sensitive_data = user_input; // Could be logged accidentally
```

### Configuration Validation
```rust
// Always validate encryption configuration at startup
if let Err(e) = config.application_encryption.validate() {
    panic!("Invalid encryption configuration: {}", e);
}
```

## Migration Patterns

### Adding Encryption to Existing Data
```rust
// 1. Add encrypted field alongside existing field
struct Entity {
    id: String,
    sensitive_data: Option<String>,           // Keep for backward compatibility
    encrypted_sensitive_data: Option<String>, // New encrypted field
}

// 2. Migration logic
async fn migrate_entity_encryption(state: &AppState, entity_id: &str) -> Result<(), Error> {
    let entity = Entity::find_by_id(&state.store, entity_id).await?;
    
    if entity.encrypted_sensitive_data.is_none() && entity.sensitive_data.is_some() {
        let identifier = Identifier::ReconMerchant(entity.merchant_id.clone());
        
        let encrypted = crypto::Encryptable::encrypt_via_api(
            state,
            Secret::new(entity.sensitive_data.unwrap()),
            identifier,
        ).await?;
        
        Entity::update(&state.store, entity_id, |e| {
            e.encrypted_sensitive_data = Some(encrypted.peek().clone());
        }).await?;
    }
    
    Ok(())
}

// 3. Access pattern with fallback
impl Entity {
    async fn get_sensitive_data(&self, state: &AppState) -> Result<String, Error> {
        if let Some(encrypted) = &self.encrypted_sensitive_data {
            let identifier = Identifier::ReconMerchant(self.merchant_id.clone());
            let decrypted = crypto::Encryptable::decrypt_via_api(
                state,
                Secret::new(encrypted.clone()),
                identifier,
            ).await?;
            Ok(decrypted.peek().clone())
        } else if let Some(legacy) = &self.sensitive_data {
            // Fallback to unencrypted data during migration
            Ok(legacy.clone())
        } else {
            Err(Error::DataNotFound)
        }
    }
}
```

## Troubleshooting Common Issues

### Configuration Problems
```bash
# Issue: "Master key must be exactly 32 bytes"
# Solution: Generate proper key
openssl rand -base64 32

# Issue: "Invalid base64 encoded string"  
# Solution: Validate key format
echo "your_key" | base64 -d

# Issue: "KeyManager certificates required"
# Solution: Ensure both cert and ca are provided in config
```

### Runtime Problems
```rust
// Issue: "Key not found in keymanager"
// Solution: Always call setup_key_for_identifier first
state.encryption_client.setup_key_for_identifier(&identifier).await?;
let encrypted = state.encryption_client.crypto_operation(/* ... */).await?;

// Issue: "Different encryption keys for same identifier"
// Solution: Ensure identifier consistency
let identifier = Identifier::ReconMerchant(merchant_id.clone()); // Consistent format
```

### Performance Issues
```rust
// Issue: High latency with KeymanagerService
// Solutions:
// 1. Use LocalEncryption for development/testing
// 2. Implement connection pooling
// 3. Batch operations when possible
// 4. Cache key existence checks (if safe for your use case)
```

## 3-Layer Architecture Pattern

<!-- AI-UPDATED: 2025-08-13 - Entries metadata encryption 3-layer pattern -->
### Overview
Used for entries and staging_entries metadata encryption. Provides clear separation between API interface, database storage, and business logic.

### Layer Definitions
```rust
// API Layer: Simple structure for external interface
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntryCreateRequest {
    // ... other fields
    pub metadata: SecretValue,  // Simple, user-friendly
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntryResponse {
    // ... other fields  
    pub metadata: EncryptableValue,  // Can be decrypted for response
}

// Diesel Layer: Complex structure for encrypted database storage
#[derive(Queryable, Identifiable)]
pub struct Entry {
    // ... other fields
    pub metadata: Encryption,  // Encrypted for database
    pub merchant_id: String,           // Required for encryption key
}

// Domain Layer: Complex structure for business logic
#[derive(Clone, Debug, serde::Serialize)]
pub struct Entry {
    // ... other fields
    pub metadata: EncryptableValue,  // Decrypted for business logic
    pub merchant_id: String,
}
```

### Conversion Pattern
```rust
#[async_trait::async_trait]
impl Conversion for Entry {
    type DstType = DieselEntry;
    
    async fn convert_back(
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError> {
        let merchant_id = item.merchant_id.clone();
        let identifier = Identifier::ReconMerchant(merchant_id.clone());

        // Decrypt metadata from database
        let encrypted_result = encryptor
            .client
            .crypto_operation(CryptoOperation::Decrypt(item.metadata), identifier)
            .await?;
        let metadata = encrypted_result.try_into_operation()?;

        Ok(Self {
            // ... map other fields
            metadata,
            merchant_id,
        })
    }
}
```

### Architecture Boundary Enforcement
```rust
// Core Layer: Handles encryption/decryption
pub async fn create_entry(
    state: &AppState,
    auth: &AuthContext,
    request: EntryCreateRequest,
) -> Result<EntryResponse, CoreError> {
    // 1. Get merchant-specific encryptor
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth.merchant_id.clone());

    // 2. Encrypt metadata
    let encrypted_metadata = merchant_encryptor
        .encrypt_json_value(request.metadata.expose())
        .await?;

    // 3. Pass encrypted context to utils layer
    let entry_new = utils::create_entry_with_metadata(
        request,
        auth.merchant_id.clone(),
        encrypted_metadata,
    ).await?;

    // 4. Store and convert back for response
    let diesel_entry = state.store.insert_entry(entry_new).await?;
    let domain_entry = DomainEntry::convert_back(&merchant_encryptor, diesel_entry).await?;
    
    Ok(domain_entry.to_response())
}

// Utils Layer: Receives encryption context
pub async fn create_entry_with_metadata(
    request: EntryCreateRequest,
    merchant_id: String,
    encrypted_metadata: Encryption,
) -> Result<EntryNew, UtilsError> {
    // Use provided encrypted metadata - no encryption logic here
    Ok(EntryNew::new(
        // ... other parameters
        merchant_id,
        encrypted_metadata,
        version,
    ))
}
```

### Hash Trait for Collections
```rust
// Required when entities are used as HashMap keys
impl std::hash::Hash for Encryption {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash the exposed inner value for deterministic behavior
        self.inner.clone().expose().hash(state);
    }
}

// Usage in reconciliation engine
let entry_mapper: HashMap<DieselEntry, EntryNew> = construct_new_entries_with_mapping(
    &remaining_entries,
    entry_status,
    version,
);
```

### Database Migration Pattern
```sql
-- 1. Add merchant_id for encryption key isolation
ALTER TABLE entries ADD COLUMN merchant_id VARCHAR(64) NOT NULL;

-- 2. Convert metadata from JSONB to BYTEA for encrypted storage
ALTER TABLE entries ALTER COLUMN metadata TYPE BYTEA 
USING convert_to(metadata::text, 'UTF8');
```

### Integration with Reconciliation Engine
```rust
// Problem: Reconciliation needs to access decrypted metadata fields
// Solution: Convert Diesel entries to Domain entries for field access

pub async fn create_confirmation_context(
    state: &AppState,
    staging_entry: DomainStagingEntry,
    account: Account,
) -> ProcessingResult<ProcessingContext> {
    // Get expected entry from database (Diesel model)
    // search_expectation links to its owning transaction via primary_transaction_id;
    // resolve to an entry id via transaction.entry_ids before this lookup.
    let diesel_expected_entry = state
        .store
        .find_entry_by_id(expected_entry_id)
        .await?;

    // Convert to domain entry for metadata field access
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(staging_entry.merchant_id.clone());

    let domain_expected_entry = DomainEntry::convert_back(
        &merchant_encryptor, 
        diesel_expected_entry.clone()
    ).await?;

    // Now can access metadata fields for business logic
    let is_matched = apply_match_rules(
        &matching_target.match_rules,
        &staging_entry,
        &domain_expected_entry,  // Uses domain model
    )?;
    
    // ... rest of logic
}
```

### Performance Optimization Notes
- Batch processing reduces encryption calls from O(n) to O(1)
- Index-based mapping avoids fragile key-value pairing
- Merchant validation ensures security without performance penalty
- Domain model conversion only when business logic needs field access

## Batch Encryption (auto-chunked)
<!-- AI-UPDATED: 2026-05-21 - Chunking via shared macro; constants resource-scoped -->
<!-- AI-UPDATED: 2026-06-07 - Batch decrypt chunked too -->

Each `TypeEncryption::batch_encrypt_chunked_via_api` impl delegates to a shared generic `chunked_batch_encrypt<Out, V, S>` free function defined at the top of `recon_domain_models::type_encryption::encrypt`. Chunks are bounded by `BATCH_ENCRYPTION_CHUNK_SIZE` and run up to `BATCH_ENCRYPTION_MAX_CONCURRENT` in flight — both constants live next to the function. Batch decrypt is chunked the same way: `batch_decrypt_chunked_via_api` delegates to `chunked_batch_decrypt<Out>`, bounded by `BATCH_DECRYPTION_CHUNK_SIZE` / `BATCH_DECRYPTION_MAX_CONCURRENT`. Callers use `EncryptionHandler::encrypt_batch_json_values` / `..._optional` unchanged; batches of any size are safe. Local key-based `batch_encrypt`/`batch_decrypt` aren't chunked (in-memory, no payload limit).

The generic `chunked()` helper lives in `common_utils/src/chunking.rs`. Numeric policy constants live with the resource that owns them so each can be tuned against its own downstream limit.