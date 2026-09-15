use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: String,
    #[schema(value_type = String)]
    pub name: Secret<String>,
    pub email: String,
}
