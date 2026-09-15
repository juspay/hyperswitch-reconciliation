use masking::Secret;

use crate::UserResponse;

/// Concrete mock user client that wraps the different implementations
#[derive(Clone)]
pub struct MockUserClient;

impl MockUserClient {
    /// Creates a mock user response for a given user_id
    pub fn create_active_mock_user(&self, user_id: &str) -> UserResponse {
        UserResponse {
            user_id: user_id.to_string(),
            name: Secret::new(format!("User {}", user_id)),
            email: format!("{}@test.com", user_id),
            is_active: true,
        }
    }
}
