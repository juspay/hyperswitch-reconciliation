use crate::id_type::MerchantId;

#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The Hyperswitch user behind the request. `None` when there is none (API key).
    pub user_id: Option<String>,
    pub merchant_id: MerchantId,
    pub profile_id: String,
}
