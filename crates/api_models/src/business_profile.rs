use common_enums::ReconProcessorStatus;
use common_types::timezone::Timezone;
use common_utils::crypto::EncryptableName;
use common_utils::id_type::MerchantId;
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BusinessProfileCreateRequest {
    #[schema(value_type = String, example = "Default Profile")]
    pub profile_name: Secret<String>,
    pub timezone: Timezone,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BusinessProfileUpdateRequest {
    #[schema(value_type = String, example = "Default Profile")]
    pub profile_name: Option<Secret<String>>,
    pub timezone: Option<Timezone>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BusinessProfileResponse {
    #[schema(example = "profile_abcdef1234")]
    pub profile_id: String,
    #[schema(value_type = String, example = "merchant_1234567890")]
    pub merchant_id: MerchantId,
    #[schema(value_type = String, example = "Default Profile")]
    pub profile_name: EncryptableName,
    pub timezone: Timezone,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReconStatusResponse {
    pub processor_status: ReconProcessorStatus,
    pub pending_staging_entries: u64,
}
