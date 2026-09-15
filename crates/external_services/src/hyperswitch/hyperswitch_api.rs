use async_trait::async_trait;
use error_stack::ResultExt;
use hyper::{body::Bytes, StatusCode};
use serde::de::DeserializeOwned;

use crate::hyperswitch::{
    client::HyperswitchClient,
    types::{HyperswitchClientError, HyperswitchResult},
};

pub mod authorize_user_token;
pub mod get_user_by_id;
pub mod list_assignees;
pub mod list_users_by_ids;

pub(crate) use authorize_user_token::AuthorizeUserToken;
pub(crate) use get_user_by_id::GetUserById;
pub(crate) use list_assignees::ListAssignees;
pub(crate) use list_users_by_ids::ListUsersByIds;

#[async_trait]
pub(crate) trait HyperswitchApi: Send + Sync {
    type Response: DeserializeOwned;

    fn get_request_builder(
        &self,
        hyperswitch_client: &HyperswitchClient,
        http_client: &reqwest::Client,
    ) -> HyperswitchResult<reqwest::RequestBuilder>;

    async fn handle_ok_response(
        &self,
        response: reqwest::Response,
    ) -> HyperswitchResult<Self::Response>;

    async fn response_handler(
        &self,
        response: reqwest::Response,
    ) -> HyperswitchResult<Self::Response> {
        match response.status() {
            StatusCode::OK => self.handle_ok_response(response).await,
            StatusCode::INTERNAL_SERVER_ERROR => {
                decode_err(response, HyperswitchClientError::InternalServerError).await
            }
            StatusCode::BAD_REQUEST => {
                decode_err(response, HyperswitchClientError::BadRequest).await
            }
            StatusCode::UNAUTHORIZED => {
                decode_err(response, HyperswitchClientError::Unauthorized).await
            }
            StatusCode::FORBIDDEN => decode_err(response, HyperswitchClientError::Forbidden).await,
            _ => decode_err(response, HyperswitchClientError::Unexpected).await,
        }
    }
}

pub(crate) async fn decode_err<R, F>(response: reqwest::Response, make: F) -> HyperswitchResult<R>
where
    F: FnOnce(Bytes) -> HyperswitchClientError,
{
    let bytes = response
        .bytes()
        .await
        .change_context(HyperswitchClientError::ResponseDecodingFailed)?;
    Err(make(bytes).into())
}
