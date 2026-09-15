use std::collections::HashSet;

use common_types::permissions::Permission;
use masking::Secret;
use serde::Serialize;

/// Request payload for batch user fetch
#[derive(Debug, Serialize)]
pub struct BatchUserRequest {
    pub user_ids: HashSet<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthorizeUserTokenRequest {
    pub token: Secret<String>,
    pub permission: Permission,
}
