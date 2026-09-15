use crate::id_type::MerchantId;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub merchant_id: MerchantId,
    pub profile_id: String,
}
