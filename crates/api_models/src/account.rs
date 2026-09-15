use common_enums::{BalanceDirection, Currency};
use common_types::{amount::AmountResponse, units::FloatMajorUnit};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request to create a new account
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountCreateRequest {
    #[schema(example = "Checking Account")]
    pub account_name: String,

    #[schema(example = "debit")]
    pub account_type: BalanceDirection,

    #[schema(example = "USD")]
    pub currency: Currency,

    #[schema(example = 1000.50)]
    pub initial_balance: FloatMajorUnit,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountUpdateRequest {
    #[schema(example = "Updated Account Name")]
    pub account_name: Option<String>,
}

/// Complete account data for flattened responses
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountData {
    #[schema(example = "account_1234567890")]
    pub account_id: String,
    #[schema(example = "Cash Account")]
    pub account_name: String,
}

/// Account response with structured amount objects
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetAccountResponse {
    pub account_id: String,
    pub account_name: String,
    pub account_type: BalanceDirection,

    /// Balance values as structured amount objects with value and currency
    pub initial_balance: AmountResponse,
    pub pending_balance: AmountResponse,
    pub posted_balance: AmountResponse,
    pub expected_balance: AmountResponse,
    pub mismatched_balance: AmountResponse,
    pub matched_balance: AmountResponse,

    /// Posted debits and credits breakdown
    #[schema(example = r#"{"value": 1500.00, "currency": "USD"}"#)]
    pub posted_debits: AmountResponse,
    #[schema(example = r#"{"value": 800.00, "currency": "USD"}"#)]
    pub posted_credits: AmountResponse,

    /// Pending debits and credits breakdown
    #[schema(example = r#"{"value": 200.00, "currency": "USD"}"#)]
    pub pending_debits: AmountResponse,
    #[schema(example = r#"{"value": 100.00, "currency": "USD"}"#)]
    pub pending_credits: AmountResponse,

    /// Expected debits and credits breakdown
    #[schema(example = r#"{"value": 300.00, "currency": "USD"}"#)]
    pub expected_debits: AmountResponse,
    #[schema(example = r#"{"value": 150.00, "currency": "USD"}"#)]
    pub expected_credits: AmountResponse,

    /// Mismatched debits and credits breakdown
    #[schema(example = r#"{"value": 50.00, "currency": "USD"}"#)]
    pub mismatched_debits: AmountResponse,
    #[schema(example = r#"{"value": 25.00, "currency": "USD"}"#)]
    pub mismatched_credits: AmountResponse,

    /// Matched debits and credits breakdown
    #[schema(example = r#"{"value": 200.00, "currency": "USD"}"#)]
    pub matched_debits: AmountResponse,
    #[schema(example = r#"{"value": 100.00, "currency": "USD"}"#)]
    pub matched_credits: AmountResponse,

    pub profile_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
}
