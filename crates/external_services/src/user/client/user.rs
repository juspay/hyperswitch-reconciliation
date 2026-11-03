use crate::{
    hyperswitch::{client::HyperswitchClient, types::HyperswitchResult},
    user::{client::mock_user_client::MockUserClient, interfaces::UserInterface},
};

#[derive(Clone)]
pub enum UserClient {
    MockUserClient(MockUserClient),
    Hyperswitch(HyperswitchClient),
}

impl UserClient {
    pub fn get_client(&self) -> &dyn UserInterface {
        match self {
            Self::MockUserClient(mock_user_client) => mock_user_client,
            Self::Hyperswitch(hyperswitch_client) => hyperswitch_client,
        }
    }

    pub async fn health_check(&self) -> Option<HyperswitchResult<()>> {
        match self {
            Self::MockUserClient(_) => None,
            Self::Hyperswitch(client) => Some(client.health_check().await),
        }
    }
}
