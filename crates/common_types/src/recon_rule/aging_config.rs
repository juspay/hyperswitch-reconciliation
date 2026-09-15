use common_utils::date_time;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use time::{Duration, PrimitiveDateTime, Time};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "aging_config_version", rename_all = "snake_case")]
pub enum AgingConfig {
    V1(AgingConfigV1),
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "aging_config_type", rename_all = "snake_case")]
pub enum AgingConfigV1 {
    #[default]
    NoAging,
    WithThreshold(AgingConfigWithThreshold),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "aging_config_with_threshold_version", rename_all = "snake_case")]
pub enum AgingConfigWithThreshold {
    V1(AgingConfigWithThresholdV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "aging_config_with_threshold_type", rename_all = "snake_case")]
pub enum AgingConfigWithThresholdV1 {
    WeekDaysV1 { value: u8 },
}

impl AgingConfig {
    pub fn check_if_missing(
        &self,
        start_date: PrimitiveDateTime,
        end_date: PrimitiveDateTime,
    ) -> bool {
        match self {
            Self::V1(config_v1) => match config_v1 {
                AgingConfigV1::NoAging => false,
                AgingConfigV1::WithThreshold(AgingConfigWithThreshold::V1(
                    AgingConfigWithThresholdV1::WeekDaysV1 { value },
                )) => {
                    let days_elapsed = date_time::weekdays_between(start_date, end_date);
                    days_elapsed > i64::from(*value)
                }
            },
        }
    }

    pub fn missing_before(&self, end_date: PrimitiveDateTime) -> Option<PrimitiveDateTime> {
        match self {
            Self::V1(config_v1) => match config_v1 {
                AgingConfigV1::NoAging => None,
                AgingConfigV1::WithThreshold(AgingConfigWithThreshold::V1(
                    AgingConfigWithThresholdV1::WeekDaysV1 { value },
                )) => {
                    let mut date = end_date.date();
                    while !is_weekday(date) {
                        date -= Duration::days(1);
                    }

                    let mut remaining_days = *value;
                    while remaining_days > 0 {
                        date -= Duration::days(1);
                        if is_weekday(date) {
                            remaining_days -= 1;
                        }
                    }

                    Some(PrimitiveDateTime::new(date, Time::MIDNIGHT))
                }
            },
        }
    }
}

fn is_weekday(date: time::Date) -> bool {
    date.weekday().number_from_monday() <= 5
}

common_utils::impl_to_sql_from_sql_json!(AgingConfig);
