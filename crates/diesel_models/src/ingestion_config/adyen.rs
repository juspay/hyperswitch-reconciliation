use common_utils::encryption::Encryption;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdyenIngestionConfig {
    pub hmac_secret: Encryption,
    pub webhook_basic_auth: AdyenBasicAuth,
    pub report_basic_auth: AdyenBasicAuth,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdyenBasicAuth {
    pub username: String,
    pub password: Encryption,
}
