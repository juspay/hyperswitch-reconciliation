use common_enums::SearchGroupStatus;
use common_utils::crypto::EncryptableName;
use common_utils::id_type::MerchantId;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchGroupResponse {
    #[schema(example = "sea_grp_1234567890")]
    pub id: String,
    #[schema(example = "rule_1234567890")]
    pub rule_id: String,
    #[schema(example = "txn_1234567890")]
    pub primary_transaction_id: String,
    #[schema(value_type = String, example = "alphanumeric")]
    pub encrypted_grouping_value: EncryptableName,
    #[schema(value_type = String, example = "alphanumeric")]
    pub hashed_grouping_value: Secret<String>,
    #[schema(value_type = String, example = "merchant_1234567890")]
    pub merchant_id: MerchantId,
    #[schema(example = "profile_1234567890")]
    pub profile_id: String,
    #[schema(example = "active")]
    pub status: SearchGroupStatus,
    #[schema(example = "2022-01-01T00:00:00")]
    pub created_at: PrimitiveDateTime,
    #[schema(example = "2022-01-01T00:00:00")]
    pub last_modified_at: PrimitiveDateTime,
}
