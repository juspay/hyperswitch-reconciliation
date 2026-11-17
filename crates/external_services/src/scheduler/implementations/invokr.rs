//! Invokr implementation

use std::time::Duration;

use async_trait::async_trait;
use aws_smithy_types::{Document, Number};
use invokr_sdk::{
    config::{retry::RetryConfig, timeout::TimeoutConfig, BehaviorVersion, Token},
    error::DisplayErrorContext,
    primitives::DateTime,
    types::TriggerTypeEnum,
    Client as InvokrSdkClient, Config as InvokrSdkConfig,
};
use masking::PeekInterface;

use crate::scheduler::{
    config::InvokrConfig,
    interfaces::{JobRequest, JobTrigger, SchedulerError, SchedulerInterface},
};

type Result<T, E> = core::result::Result<T, error_stack::Report<E>>;

const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct InvokrClient {
    client: InvokrSdkClient,
    org_id: String,
    workspace_id: String,
}

impl InvokrClient {
    pub fn new(config: &InvokrConfig) -> Self {
        let timeout = Duration::from_secs(
            config
                .request_timeout_secs
                .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS),
        );
        let sdk_config = InvokrSdkConfig::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(config.base_url.clone())
            .bearer_token(Token::new(config.api_token.peek().clone(), None))
            .timeout_config(TimeoutConfig::builder().operation_timeout(timeout).build())
            // Invokr gives a job without an idempotency key a fresh one, so a retry
            // could create the job twice.
            .retry_config(RetryConfig::disabled())
            .build();

        Self {
            client: InvokrSdkClient::from_conf(sdk_config),
            org_id: config.org_id.clone(),
            workspace_id: config.workspace_id.clone(),
        }
    }
}

#[async_trait]
impl SchedulerInterface for InvokrClient {
    async fn create_job(&self, request: JobRequest) -> Result<String, SchedulerError> {
        let input = request
            .input
            .map(|input| match input {
                serde_json::Value::Object(_) => Ok(json_to_document(input)),
                _ => Err(error_stack::Report::new(SchedulerError::InvalidRequest)
                    .attach("Invokr job input must be a JSON object")),
            })
            .transpose()?;

        let builder = self
            .client
            .create_job()
            .org_id(&self.org_id)
            .workspace_id(&self.workspace_id)
            .endpoint(&request.endpoint)
            .set_idempotency_key(request.idempotency_key)
            .set_input(input);

        let builder = match request.trigger {
            JobTrigger::Immediate => builder.trigger(TriggerTypeEnum::Immediate),
            JobTrigger::Delayed { run_at } => {
                let run_at = run_at.assume_utc();
                builder
                    .trigger(TriggerTypeEnum::Delayed)
                    .run_at(DateTime::from_secs_and_nanos(
                        run_at.unix_timestamp(),
                        run_at.nanosecond(),
                    ))
            }
            JobTrigger::Cron {
                expression,
                timezone,
            } => builder
                .trigger(TriggerTypeEnum::Cron)
                .cron(expression)
                .timezone(timezone),
        };

        // Without `DisplayErrorContext`, every SDK error prints as "service error".
        let output = builder.send().await.map_err(|err| {
            error_stack::Report::new(SchedulerError::JobCreationFailed).attach(format!(
                "Invokr create job failed: {}: {}",
                request.endpoint,
                DisplayErrorContext(&err)
            ))
        })?;

        let job_id = output.data().job_id().to_string();
        tracing::info!(
            endpoint = request.endpoint,
            job_id = job_id,
            "Invokr job created"
        );
        Ok(job_id)
    }
}

fn json_to_document(value: serde_json::Value) -> Document {
    match value {
        serde_json::Value::Null => Document::Null,
        serde_json::Value::Bool(boolean) => Document::Bool(boolean),
        serde_json::Value::Number(number) => Document::Number(
            number
                .as_u64()
                .map(Number::PosInt)
                .or_else(|| number.as_i64().map(Number::NegInt))
                .or_else(|| number.as_f64().map(Number::Float))
                .unwrap_or(Number::Float(f64::NAN)),
        ),
        serde_json::Value::String(string) => Document::String(string),
        serde_json::Value::Array(values) => {
            Document::Array(values.into_iter().map(json_to_document).collect())
        }
        serde_json::Value::Object(map) => Document::Object(
            map.into_iter()
                .map(|(key, value)| (key, json_to_document(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn converts_nested_json_to_document() {
        let document = json_to_document(serde_json::json!({
            "report_id": "report_123",
            "count": 2,
            "offset": -1,
            "ratio": 0.5,
            "tags": ["a", null, true],
        }));

        let expected = Document::Object(HashMap::from([
            (
                "report_id".to_string(),
                Document::String("report_123".into()),
            ),
            ("count".to_string(), Document::Number(Number::PosInt(2))),
            ("offset".to_string(), Document::Number(Number::NegInt(-1))),
            ("ratio".to_string(), Document::Number(Number::Float(0.5))),
            (
                "tags".to_string(),
                Document::Array(vec![
                    Document::String("a".into()),
                    Document::Null,
                    Document::Bool(true),
                ]),
            ),
        ]));
        assert_eq!(document, expected);
    }
}
