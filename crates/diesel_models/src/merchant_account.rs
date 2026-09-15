use common_utils::{encryption::Encryption, id_type::MerchantId};
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::merchant_account;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccount {
    pub merchant_id: MerchantId,
    pub merchant_name: Encryption,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = merchant_account)]
pub struct MerchantAccountNew {
    pub merchant_id: MerchantId,
    pub merchant_name: Encryption,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}
