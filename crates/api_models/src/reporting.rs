use std::collections::HashSet;

use common_types::date_time::{TimeRange, TimeRangeValidation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct TransactionReportFilters {
    #[serde(flatten)]
    pub time_range: TimeRange,
    pub rule_id: String,
    #[serde(default)]
    pub emails: HashSet<String>,
}

impl TransactionReportFilters {
    pub fn validate(&self) -> Result<(), String> {
        self.time_range.validate()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ExceptionReportFilters {
    #[serde(flatten)]
    pub time_range: TimeRange,
    pub rule_id: String,
    #[serde(default)]
    pub emails: HashSet<String>,
}

impl ExceptionReportFilters {
    pub fn validate(&self) -> Result<(), String> {
        self.time_range.validate()
    }
}
