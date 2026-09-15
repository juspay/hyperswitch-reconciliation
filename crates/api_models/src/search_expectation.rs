use common_enums::ExpectationStatus;
use common_types::field_path_key::FieldPathKey;
use common_utils::{crypto::EncryptableName, id_type::MerchantId};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchExpectationResponse {
    #[schema(example = "sea_exp_1234567890")]
    pub id: String,
    #[schema(example = "txn_1234567890")]
    pub primary_transaction_id: String,
    #[schema(example = "rule_1234567890")]
    pub rule_id: String,
    #[schema(example = "acc_1234567890")]
    pub target_account_id: String,
    pub target_key: FieldPathKey,
    #[schema(example = "expected")]
    pub status: ExpectationStatus,
    #[schema(value_type = String, example = "alphanumeric")]
    pub encrypted_source_value: EncryptableName,
    #[schema(value_type = String, example = "alphanumeric")]
    pub hashed_source_value: Secret<String>,
    #[schema(value_type = String, example = "merchant_1234567890")]
    pub merchant_id: MerchantId,
    #[schema(example = "2022-01-01T00:00:00")]
    pub created_at: PrimitiveDateTime,
    #[schema(example = "2022-01-01T00:00:00")]
    pub last_modified_at: PrimitiveDateTime,
}
