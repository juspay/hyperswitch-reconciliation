use common_utils::{encryption::Encryption, id_type::MerchantId};
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::business_profile;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = business_profile)]
pub struct BusinessProfile {
    pub profile_id: String,
    pub profile_name: Encryption,
    pub merchant_id: MerchantId,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = business_profile)]
pub struct BusinessProfileNew {
    pub profile_id: String,
    pub profile_name: Encryption,
    pub merchant_id: MerchantId,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}
