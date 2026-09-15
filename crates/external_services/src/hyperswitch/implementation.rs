use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use common_utils::{errors::ReportSwitchExt, id_type::MerchantId};
use recon_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::{
    hyperswitch::{
        client::HyperswitchClient,
        config::HyperswitchConfig,
        hyperswitch_api::{AuthorizeUserToken, GetUserById, ListAssignees, ListUsersByIds},
        types::EntityType,
    },
    user::{
        interfaces::UserInterface,
        types::{
            error::UserClientResult,
            request::{AuthorizeUserTokenRequest, BatchUserRequest},
        },
    },
    UserResponse,
};

#[async_trait]
impl UserInterface for HyperswitchClient {
    async fn get_user_by_id(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        user_id: String,
    ) -> UserClientResult<UserResponse> {
        self.send_hyperswitch_request(GetUserById {
            user_id,
            merchant_id: merchant_id.clone(),
            profile_id: profile_id.to_string(),
            internal_api_key: self.internal_api_key.clone(),
        })
        .await
        .switch()
    }

    async fn list_users_by_ids(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        user_ids: HashSet<String>,
    ) -> UserClientResult<HashMap<String, UserResponse>> {
        let response = self
            .send_hyperswitch_request(ListUsersByIds {
                payload: BatchUserRequest { user_ids },
                merchant_id: merchant_id.clone(),
                profile_id: profile_id.to_string(),
                internal_api_key: self.internal_api_key.clone(),
            })
            .await
            .switch()?;
        Ok(response
            .users
            .into_iter()
            .map(|user| (user.user_id.clone(), user))
            .collect())
    }

    async fn list_assignees(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
    ) -> UserClientResult<Vec<UserResponse>> {
        self.send_hyperswitch_request(ListAssignees {
            access_type: EntityType::Profile,
            merchant_id: merchant_id.clone(),
            profile_id: profile_id.to_string(),
            internal_api_key: self.internal_api_key.clone(),
        })
        .await
        .switch()
    }

    async fn authorize_user_token(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        payload: AuthorizeUserTokenRequest,
    ) -> UserClientResult<()> {
        self.send_hyperswitch_request(AuthorizeUserToken {
            payload,
            merchant_id: merchant_id.clone(),
            profile_id: profile_id.to_string(),
            internal_api_key: self.internal_api_key.clone(),
        })
        .await
        .switch()
    }
}

#[async_trait]
impl SecretsHandler for HyperswitchConfig {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> error_stack::Result<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        // Store values we need before moving
        let hyperswitch_config = value.get_inner();
        let base_url = hyperswitch_config.base_url.clone();
        let request_timeout_secs = hyperswitch_config.request_timeout_secs;
        let client_idle_timeout = hyperswitch_config.client_idle_timeout;

        // Decrypt the internal API key using KMS
        let decrypted_api_key = secret_management_client
            .get_secret(hyperswitch_config.internal_api_key.clone())
            .await?;

        Ok(value.transition_state(|_| Self {
            base_url,
            internal_api_key: decrypted_api_key,
            request_timeout_secs,
            client_idle_timeout,
        }))
    }
}
