use crate::id_type::MerchantId;

/// How the request authenticated. An API key has no Hyperswitch user, so code that resolves
/// `user_id` through Hyperswitch (e.g. `modified_by`) must branch on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    Jwt,
    ApiKey,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub auth_method: AuthMethod,
}
