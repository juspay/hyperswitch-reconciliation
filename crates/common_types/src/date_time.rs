use common_utils::custom_serde::iso8601;
use time::PrimitiveDateTime;

/// A type representing a range of time for filtering, including a mandatory start time and an optional end time.
#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema,
)]
pub struct TimeRange {
    /// The start time to filter payments list or to get list of filters. To get list of filters start time is needed to be passed
    #[serde(with = "iso8601")]
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    pub start_time: PrimitiveDateTime,
    /// The end time to filter payments list or to get list of filters. If not passed the default time is now
    #[serde(default, with = "iso8601::option")]
    #[schema(value_type = Option<String>, example = "2024-12-31T23:59:59Z")]
    pub end_time: Option<PrimitiveDateTime>,
}

#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema,
)]
pub struct DefinedTimeRange {
    #[serde(with = "iso8601")]
    #[schema(value_type = String, example = "2024-01-01T00:00:00Z")]
    pub start_time: PrimitiveDateTime,
    #[serde(with = "iso8601")]
    #[schema(value_type = String, example = "2024-12-31T23:59:59Z")]
    pub end_time: PrimitiveDateTime,
}

pub trait TimeRangeValidation {
    fn start_time(&self) -> PrimitiveDateTime;
    fn end_time(&self) -> Option<PrimitiveDateTime>;

    fn validate(&self) -> Result<(), String> {
        if let Some(end_time) = self.end_time() {
            if self.start_time() > end_time {
                return Err("start_time cannot be greater than end_time".to_string());
            }
        }
        Ok(())
    }
}

impl TimeRangeValidation for TimeRange {
    fn start_time(&self) -> PrimitiveDateTime {
        self.start_time
    }

    fn end_time(&self) -> Option<PrimitiveDateTime> {
        self.end_time
    }
}

impl TimeRangeValidation for DefinedTimeRange {
    fn start_time(&self) -> PrimitiveDateTime {
        self.start_time
    }

    fn end_time(&self) -> Option<PrimitiveDateTime> {
        Some(self.end_time)
    }
}

impl TimeRange {
    pub fn into_defined(self, default_end_time: PrimitiveDateTime) -> DefinedTimeRange {
        DefinedTimeRange {
            start_time: self.start_time,
            end_time: self.end_time.unwrap_or(default_end_time),
        }
    }
}

impl From<DefinedTimeRange> for TimeRange {
    fn from(value: DefinedTimeRange) -> Self {
        Self {
            start_time: value.start_time,
            end_time: Some(value.end_time),
        }
    }
}
