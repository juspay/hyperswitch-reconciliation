use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdyenTriggerData {
    pub live: String,
    pub notification_items: Vec<NotificationItemWrapper>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItemWrapper {
    #[serde(rename = "NotificationRequestItem")]
    pub notification_request_item: NotificationRequestItem,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRequestItem {
    pub amount: Amount,
    pub event_code: EventCode,
    pub event_date: String,
    pub merchant_account_code: String,
    pub merchant_reference: String,
    pub original_reference: Option<String>,
    pub psp_reference: String,
    pub reason: String,
    pub success: String,
    pub additional_data: AdditionalData,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalData {
    pub hmac_signature: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventCode {
    ReportAvailable,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Amount {
    pub currency: String,
    pub value: i64,
}
