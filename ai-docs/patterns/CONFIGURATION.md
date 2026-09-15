# Configuration Patterns

## Environment Variable Configuration

### Basic Setup

Environment variables use the prefix `RECON__` with double underscores (`__`) separating nested levels:

```bash
# Database configuration
export RECON__DATABASE__HOST="localhost"
export RECON__DATABASE__PORT=5432
export RECON__DATABASE__USERNAME="user"
export RECON__DATABASE__PASSWORD="pass"

# Server configuration  
export RECON__SERVER__HOST="0.0.0.0"
export RECON__SERVER__PORT=8080
export RECON__SERVER__WORKERS=4
```

### Configuration Priority Order

1. **Environment Variables** (highest priority)
2. **Config Files** (`config/config.toml`, `config/prod.toml`)
3. **Default Values** (lowest priority)

### Environment Detection

Use `RUN_ENV` to control which config file is loaded:
- `RUN_ENV=production` → loads `config/prod.toml`
- `RUN_ENV=development` or unset → loads `config/config.toml`

### Command Line Options

```bash
# Custom config file path
./recon -f /path/to/config.toml
./recon --config-path /path/to/config.toml
```

## Common Gotchas

<!-- AI-UPDATED: 2025-01-07 - Environment variable parsing with list_separator -->
- Environment variables with `list_separator(",")` can cause "sequence vs string" parsing errors
- The config crate tries to parse comma-containing values as arrays even for string fields
- Keep `list_separator(",")` only if you have actual array fields that need comma-separated values else not required
- For simple string environment variables, this can cause confusing "invalid type: sequence, expected a string" errors

## Environment Variable Override Gotchas
<!-- AI-UPDATED: 2025-01-07 - Fixed environment variable override bug -->
- Environment variables were not overriding config file values due to dual config loading
- `initialize_server()` was loading config directly from TOML file, ignoring env vars
- **Fix**: Modified `initialize_server()` to accept pre-loaded config object that respects env vars
- **Example**: `RECON__SERVER__PORT=8081` now correctly overrides config.toml port value

## Working Environment Variable Format
<!-- AI-UPDATED: 2025-01-07 - Documented working env var patterns -->
- Use `RECON__` prefix with double underscores for nesting
- Enum values must be lowercase: `RECON__LOGGING__FORMAT=pretty` (not `Pretty`)
- Boolean values: `RECON__LOGGING__CONSOLE_OUTPUT=true`
- Numeric values: `RECON__SERVER__PORT=8081`
- Tagged enums like `SecretsManagementConfig` may not work via env vars - use config file


### Data Type Formatting

```bash
# Correct formatting for different types
export RECON__SERVER__PORT=8080           # Numbers without quotes
export RECON__LOGGING__CONSOLE_OUTPUT=true  # Booleans without quotes
export RECON__DATABASE__HOST="localhost"    # Strings with quotes
```

### Secrets Management

```bash
# Development (no encryption)
export RECON__SECRETS_MANAGEMENT__SECRETS_MANAGER="no_encryption"

# Production (AWS KMS)
export RECON__SECRETS_MANAGEMENT__SECRETS_MANAGER="aws_kms"
export RECON__SECRETS_MANAGEMENT__AWS_KMS__KEY_ID="arn:aws:kms:..."
export RECON__SECRETS_MANAGEMENT__AWS_KMS__REGION="us-east-1"
```

### Transformation Service Configuration
<!-- AI-UPDATED: 2025-07-21 - Manual upload transformation service integration -->
```bash
# Transformation service base URL for ingestion-to-transformation HTTP calls
export RECON__TRANSFORMATION_SERVICE__BASE_URL="http://localhost:8080"

# In production, use appropriate service discovery URL
export RECON__TRANSFORMATION_SERVICE__BASE_URL="http://transformation-service:8080"
```

## Deployment Examples

### Docker
```bash
docker run -e RECON__DATABASE__HOST=db \
           -e RECON__SERVER__PORT=8080 \
           hyperswitch-recon
```

### Kubernetes
```yaml
env:
- name: RECON__DATABASE__HOST
  value: "postgres-service"
- name: RECON__DATABASE__PASSWORD
  valueFrom:
    secretKeyRef:
      name: db-secret
      key: password
```

### Application Encryption Configuration
<!-- AI-UPDATED: 2025-01-29 - Encryption system configuration -->

#### Local Encryption (Development)
```bash
export RECON__APPLICATION_ENCRYPTION__ENCRYPTION_MODE="local_encryption"
export RECON__APPLICATION_ENCRYPTION__LOCAL_ENCRYPTION__MASTER_KEY="$(openssl rand -base64 32)"
```

#### KeyManager Service (Production)
```bash
export RECON__APPLICATION_ENCRYPTION__ENCRYPTION_MODE="key_manager"
export RECON__APPLICATION_ENCRYPTION__KEY_MANAGER__ENABLED=true
export RECON__APPLICATION_ENCRYPTION__KEY_MANAGER__URL="https://keymanager.prod.com"
# Note: Certificate configuration typically handled via config files
```

#### Configuration File Examples

**Local Development:**
```toml
[application_encryption]
encryption_mode = "local_encryption"

[application_encryption.local_encryption]
master_key = "your_base64_encoded_32_byte_key"
```

**Production:**
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

#### Master Key Generation
```bash
# Generate a secure 32-byte master key
openssl rand -base64 32

# Validate key format (should output 32)
echo "your_key_here" | base64 -d | wc -c
```

### Local Development
```bash
# Override specific values while keeping defaults
RECON__DATABASE__HOST="localhost" \
RECON__SERVER__PORT=8081 \
cargo run
