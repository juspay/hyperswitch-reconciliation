//! HyperSwitch interfaces and data models

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use common_utils::id_type::MerchantId;

use crate::user::types::{
    error::UserClientResult, request::AuthorizeUserTokenRequest, response::UserResponse,
};

#[async_trait]
pub trait UserInterface {
    async fn get_user_by_id(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        user_id: String,
    ) -> UserClientResult<UserResponse>;
    async fn list_users_by_ids(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        user_ids: HashSet<String>,
    ) -> UserClientResult<HashMap<String, UserResponse>>;
    async fn list_assignees(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
    ) -> UserClientResult<Vec<UserResponse>>;
    async fn authorize_user_token(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        payload: AuthorizeUserTokenRequest,
    ) -> UserClientResult<()>;
}
