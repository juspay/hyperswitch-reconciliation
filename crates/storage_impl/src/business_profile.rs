use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use diesel_models::{
    business_profile::{BusinessProfile, BusinessProfileNew, BusinessProfileUpdate},
    StorageResult,
};

use crate::database::store::{DatabaseRead, DatabaseWrite};

#[async_trait]
pub trait ProfileReadInterface {
    async fn list_business_profiles_by_merchant_id(
        &self,
        merchant_id: MerchantId,
    ) -> StorageResult<Vec<BusinessProfile>>;

    async fn find_business_profile_by_id(&self, profile_id: &str)
        -> StorageResult<BusinessProfile>;
}

#[async_trait]
pub trait ProfileWriteInterface {
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

#[async_trait]
impl<T: DatabaseRead + Sync> ProfileReadInterface for T {
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

#[async_trait]
impl<T: DatabaseWrite + Sync> ProfileWriteInterface for T {
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
