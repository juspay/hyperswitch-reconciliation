/// Default tenant to be used when multitenancy is disabled
pub const TENANT_HEADER: &str = "x-tenant-id";

/// Default tenant
pub const DEFAULT_TENANT_ID: &str = "public";

/// General purpose base64 engine
pub const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// API client request timeout (in seconds)
pub const REQUEST_TIME_OUT: u64 = 30;

/// Maximum allowed length for merchant reference ID
pub const MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH: u8 = 64;

/// Minimum required length for merchant reference ID
pub const MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH: u8 = 1;
