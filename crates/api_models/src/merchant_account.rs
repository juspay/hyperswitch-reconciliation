use common_utils::crypto::EncryptableName;
use common_utils::id_type::MerchantId;
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MerchantAccountCreateRequest {
    #[schema(value_type = String, example = "Acme Corporation")]
    pub merchant_name: Secret<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MerchantAccountResponse {
    #[schema(value_type = String, example = "merchant_1234567890")]
    pub merchant_id: MerchantId,
    #[schema(value_type = String, example = "Acme Corporation")]
    pub merchant_name: EncryptableName,
}
