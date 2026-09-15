use common_utils::id_type::MerchantId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TestMerchantSetupRequest {
    pub merchant_name: String,
    pub profile_name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TestMerchantSetupResponse {
    #[schema(value_type = String)]
    pub merchant_id: MerchantId,
    pub profile_id: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TestMerchantSetupWithIdsRequest {
    #[schema(value_type = String)]
    pub merchant_id: MerchantId,
    pub merchant_name: String,
    pub profile_id: String,
    pub profile_name: String,
}
