use common_utils::custom_serde::iso8601;
use serde::Serialize;
use time::PrimitiveDateTime;

#[derive(Serialize)]
pub struct TransactionReportLambdaPayload {
    pub profile_id: String,
    pub rule_id: String,
    #[serde(with = "iso8601")]
    pub start_time: PrimitiveDateTime,
    #[serde(with = "iso8601")]
    pub end_time: PrimitiveDateTime,
    pub emails: Vec<String>,
    #[serde(with = "iso8601::option")]
    pub aging_less_than: Option<PrimitiveDateTime>,
}

#[derive(Serialize)]
pub struct ExceptionReportLambdaPayload {
    pub profile_id: String,
    pub rule_id: String,
    #[serde(with = "iso8601")]
    pub start_time: PrimitiveDateTime,
    #[serde(with = "iso8601")]
    pub end_time: PrimitiveDateTime,
    pub emails: Vec<String>,
    #[serde(with = "iso8601::option")]
    pub aging_less_than: Option<PrimitiveDateTime>,
}
