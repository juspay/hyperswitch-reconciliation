use crate::id_type::MerchantId;

#[derive(Debug, Clone)]
pub enum Actor {
    User { user_id: String },
}

impl Actor {
    pub fn user_id(&self) -> Option<&str> {
        match self {
            Self::User { user_id } => Some(user_id),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub merchant_id: MerchantId,
    pub profile_id: String,
    pub actor: Actor,
}

impl AuthContext {
    pub fn user_id(&self) -> Option<String> {
        self.actor.user_id().map(str::to_owned)
    }
}
