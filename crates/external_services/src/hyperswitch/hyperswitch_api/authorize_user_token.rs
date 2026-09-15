use std::time::Duration;

use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use hyper::{HeaderMap, Method};
use masking::Secret;

use crate::{
    hyperswitch::{
        client::HyperswitchClient,
        hyperswitch_api::HyperswitchApi,
        types::{HyperswitchAuth, HyperswitchPath, HyperswitchResult, StandardHeader},
    },
    user::types::request::AuthorizeUserTokenRequest,
};

pub(crate) struct AuthorizeUserToken {
    pub payload: AuthorizeUserTokenRequest,
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub internal_api_key: Secret<String>,
}

#[async_trait]
impl HyperswitchApi for AuthorizeUserToken {
    type Response = ();

    fn get_request_builder(
        &self,
        hyperswitch_client: &HyperswitchClient,
        http_client: &reqwest::Client,
    ) -> HyperswitchResult<reqwest::RequestBuilder> {
        let path = HyperswitchPath::AuthorizeUserToken.to_path(hyperswitch_client.base_url.clone());

        let auth = HyperswitchAuth::InternalApiKey {
            internal_api_key: self.internal_api_key.clone(),
            merchant_id: self.merchant_id.clone(),
            profile_id: self.profile_id.clone(),
        };

        let mut headers = vec![StandardHeader::TenantIdPublic.pair()];
        headers.extend(auth.get_auth_headers()?);

        let request_builder = http_client
            .request(Method::POST, path)
            .headers(HeaderMap::from_iter(headers))
            .json(&self.payload)
            .timeout(Duration::from_secs(
                hyperswitch_client.request_timeout_secs.unwrap_or(30),
            ));

        Ok(request_builder)
    }

    async fn handle_ok_response(
        &self,
        _response: reqwest::Response,
    ) -> HyperswitchResult<Self::Response> {
        Ok(())
    }
}
