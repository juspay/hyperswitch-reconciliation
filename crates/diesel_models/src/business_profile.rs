use common_types::timezone::Timezone;
use common_utils::{encryption::Encryption, id_type::MerchantId};
use diesel::{AsChangeset, Insertable, Queryable};
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
    pub timezone: Timezone,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = business_profile)]
pub struct BusinessProfileNew {
    pub profile_id: String,
    pub profile_name: Encryption,
    pub merchant_id: MerchantId,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub timezone: Timezone,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = business_profile)]
pub struct BusinessProfileUpdateInternal {
    pub profile_name: Option<Encryption>,
    pub timezone: Option<Timezone>,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Debug, Clone)]
pub enum BusinessProfileUpdate {
    NameUpdate { profile_name: Encryption },
    TimezoneUpdate { timezone: Timezone },
}

impl From<BusinessProfileUpdate> for BusinessProfileUpdateInternal {
    fn from(business_profile_update: BusinessProfileUpdate) -> Self {
        let last_modified_at = common_utils::date_time::now();

        match business_profile_update {
            BusinessProfileUpdate::NameUpdate { profile_name } => Self {
                profile_name: Some(profile_name),
                timezone: None,
                last_modified_at,
            },
            BusinessProfileUpdate::TimezoneUpdate { timezone } => Self {
                profile_name: None,
                timezone: Some(timezone),
                last_modified_at,
            },
        }
    }
}
