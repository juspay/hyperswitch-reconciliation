use masking::Secret;
use serde::{Deserialize, Serialize};

/// User information from HyperSwitch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub user_id: String,
    pub name: Secret<String>,
    pub email: String,
    pub is_active: bool,
}

/// Response from HyperSwitch batch user endpoint
#[derive(Debug, Deserialize)]
pub struct BatchUserResponse {
    pub users: Vec<UserResponse>,
}
