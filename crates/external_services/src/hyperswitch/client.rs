use std::time::Duration;

use error_stack::ResultExt;
use masking::Secret;
use once_cell::sync::OnceCell;

use crate::hyperswitch::{
    hyperswitch_api::HyperswitchApi,
    types::{HyperswitchClientError, HyperswitchResult},
};

/// HyperSwitch client that makes actual HTTP calls to HyperSwitch
#[derive(Clone)]
pub struct HyperswitchClient {
    pub base_url: String,
    pub internal_api_key: Secret<String>,
    pub request_timeout_secs: Option<u64>,
    pub client_idle_timeout: Option<u64>,
}

static HYPERSWITCH_API_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

impl HyperswitchClient {
    fn get_api_hyperswitch_client(&self) -> HyperswitchResult<reqwest::Client> {
        let get_client = || {
            let mut client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .pool_idle_timeout(Duration::from_secs(self.client_idle_timeout.unwrap_or(90)));

            client = client.use_rustls_tls();

            client
                .build()
                .change_context(HyperswitchClientError::ClientConstructionFailed)
        };

        Ok(HYPERSWITCH_API_CLIENT.get_or_try_init(get_client)?.clone())
    }

    pub(crate) async fn send_hyperswitch_request<E: HyperswitchApi>(
        &self,
        hyperswitch_api: E,
    ) -> HyperswitchResult<E::Response> {
        let client = self.get_api_hyperswitch_client()?;

        let request = hyperswitch_api.get_request_builder(self, &client)?;

        let response =
            request
                .send()
                .await
                .change_context(HyperswitchClientError::RequestNotSent(
                    "Unable to send request to hyperswitch".to_string(),
                ))?;

        tracing::info!(hyperswitch_response=?response);

        hyperswitch_api.response_handler(response).await
    }
}
