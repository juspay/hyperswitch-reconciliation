use crate::{
    hyperswitch::client::HyperswitchClient,
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
}
