use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use diesel_models::{
    business_profile::{BusinessProfile, BusinessProfileNew, BusinessProfileUpdate},
    StorageResult,
};

use crate::database::connection::{DatabaseRead, DatabaseWrite};

#[async_trait]
pub trait BusinessProfileReadInterface {
    async fn list_business_profiles_by_merchant_id(
        &self,
        merchant_id: MerchantId,
    ) -> StorageResult<Vec<BusinessProfile>>;

    async fn find_business_profile_by_id(&self, profile_id: &str)
        -> StorageResult<BusinessProfile>;
}

#[async_trait]
pub trait BusinessProfileWriteInterface {
    async fn insert_business_profile(
        &self,
        business_profile: BusinessProfileNew,
    ) -> StorageResult<BusinessProfile>;

    async fn update_business_profile(
        &self,
        profile_id: &str,
        business_profile_update: BusinessProfileUpdate,
    ) -> StorageResult<BusinessProfile>;
}

/// Any store that can hand out read connections can read business profiles.
#[async_trait]
impl<T: DatabaseRead> BusinessProfileReadInterface for T {
    async fn list_business_profiles_by_merchant_id(
        &self,
        merchant_id: MerchantId,
    ) -> StorageResult<Vec<BusinessProfile>> {
        let conn = self.read_conn().await?;
        BusinessProfile::list_by_merchant_id(&conn, merchant_id).await
    }

    async fn find_business_profile_by_id(
        &self,
        profile_id: &str,
    ) -> StorageResult<BusinessProfile> {
        let conn = self.read_conn().await?;
        BusinessProfile::find_by_profile_id(&conn, profile_id).await
    }
}

/// Any store that can hand out write connections can write business profiles.
#[async_trait]
impl<T: DatabaseWrite> BusinessProfileWriteInterface for T {
    async fn insert_business_profile(
        &self,
        business_profile: BusinessProfileNew,
    ) -> StorageResult<BusinessProfile> {
        let conn = self.write_conn().await?;
        business_profile.insert(&conn).await
    }

    async fn update_business_profile(
        &self,
        profile_id: &str,
        business_profile_update: BusinessProfileUpdate,
    ) -> StorageResult<BusinessProfile> {
        let conn = self.write_conn().await?;
        BusinessProfile::update_by_profile_id(&conn, profile_id, business_profile_update.into())
            .await
    }
}
