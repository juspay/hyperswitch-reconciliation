use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use masking::{PeekInterface, Secret};
use recon_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::{
    user::{
        interfaces::UserInterface,
        types::{error::UserClientResult, request::AuthorizeUserTokenRequest},
    },
    HyperswitchConfig, UserClient, UserConfig, UserResponse,
};

#[async_trait]
impl UserInterface for UserClient {
    async fn get_user_by_id(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        user_id: String,
    ) -> UserClientResult<UserResponse> {
        self.get_client()
            .get_user_by_id(merchant_id, profile_id, user_id)
            .await
    }

    async fn list_users_by_ids(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        user_ids: HashSet<String>,
    ) -> UserClientResult<HashMap<String, UserResponse>> {
        self.get_client()
            .list_users_by_ids(merchant_id, profile_id, user_ids)
            .await
    }

    async fn list_assignees(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
    ) -> UserClientResult<Vec<UserResponse>> {
        self.get_client()
            .list_assignees(merchant_id, profile_id)
            .await
    }

    async fn authorize_user_token(
        &self,
        merchant_id: MerchantId,
        profile_id: String,
        payload: AuthorizeUserTokenRequest,
    ) -> UserClientResult<()> {
        self.get_client()
            .authorize_user_token(merchant_id, profile_id, payload)
            .await
    }
}

#[async_trait]
impl SecretsHandler for UserConfig {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> error_stack::Result<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let config = value.get_inner();

        match config {
            Self::MockUserClient => {
                // No secrets to decrypt for local service
                Ok(value.transition_state(|_| Self::MockUserClient))
            }
            Self::Hyperswitch { hyperswitch_config } => {
                // Store values we need before moving
                let base_url = hyperswitch_config.base_url.clone();
                let request_timeout_secs = hyperswitch_config.request_timeout_secs;
                let client_idle_timeout = hyperswitch_config.client_idle_timeout;
                // Decrypt the internal API key using KMS
                let decrypted_api_key = secret_management_client
                    .get_secret(hyperswitch_config.internal_api_key.clone())
                    .await?;

                Ok(value.transition_state(|_| Self::Hyperswitch {
                    hyperswitch_config: HyperswitchConfig {
                        base_url,
                        internal_api_key: Secret::new(decrypted_api_key.peek().clone()),
                        request_timeout_secs,
                        client_idle_timeout,
                    },
                }))
            }
        }
    }
}
