use api_models::user::User as ApiUser;
use external_services::UserResponse as ExternalUserResponse;

use crate::types::transformers::ForeignFrom;

impl ForeignFrom<ExternalUserResponse> for ApiUser {
    fn foreign_from(user: ExternalUserResponse) -> Self {
        Self {
            id: user.user_id,
            name: user.name,
            email: user.email,
        }
    }
}
