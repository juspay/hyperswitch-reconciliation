use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use common_utils::id_type::MerchantId;

use crate::{
    user::{
        client::mock_user_client::MockUserClient,
        interfaces::UserInterface,
        types::{error::UserClientResult, request::AuthorizeUserTokenRequest},
    },
    UserResponse,
};

#[async_trait]
impl UserInterface for MockUserClient {
    async fn get_user_by_id(
        &self,
        _merchant_id: MerchantId,
        _profile_id: String,
        user_id: String,
    ) -> UserClientResult<UserResponse> {
        // Return mock user data
        Ok(self.create_active_mock_user(&user_id))
    }

    async fn list_users_by_ids(
        &self,
        _merchant_id: MerchantId,
        _profile_id: String,
        user_ids: HashSet<String>,
    ) -> UserClientResult<HashMap<String, UserResponse>> {
        // Return mock data for all requested user IDs
        let user_map = user_ids
            .into_iter()
            .map(|user_id| {
                let user = self.create_active_mock_user(&user_id);
                (user_id, user)
            })
            .collect();

        Ok(user_map)
    }

    async fn list_assignees(
        &self,
        _merchant_id: MerchantId,
        _profile_id: String,
    ) -> UserClientResult<Vec<UserResponse>> {
        Ok(vec![
            self.create_active_mock_user("123-456-789"),
            self.create_active_mock_user("987-654-321"),
        ])
    }

    async fn authorize_user_token(
        &self,
        _merchant_id: MerchantId,
        _profile_id: String,
        _payload: AuthorizeUserTokenRequest,
    ) -> UserClientResult<()> {
        // For the mock, we can assume any token is authorized
        Ok(())
    }
}
