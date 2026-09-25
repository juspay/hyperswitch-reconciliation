use async_bb8_diesel::AsyncConnection;
use async_trait::async_trait;
use common_utils::id_type::MerchantId;
use diesel_models::{
    business_profile::{BusinessProfile, BusinessProfileNew, BusinessProfileUpdate},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait BusinessProfileInterface {
    async fn list_business_profiles_by_merchant_id(
        &self,
        merchant_id: MerchantId,
    ) -> StorageResult<Vec<BusinessProfile>>;

    async fn insert_business_profile(
        &self,
        business_profile: BusinessProfileNew,
    ) -> StorageResult<BusinessProfile>;

    async fn find_business_profile_by_id(&self, profile_id: &str)
        -> StorageResult<BusinessProfile>;

    async fn update_business_profile(
        &self,
        profile_id: &str,
        business_profile_update: BusinessProfileUpdate,
    ) -> StorageResult<BusinessProfile>;
}

#[async_trait]
impl BusinessProfileInterface for Store {
    async fn list_business_profiles_by_merchant_id(
        &self,
        merchant_id: MerchantId,
    ) -> StorageResult<Vec<BusinessProfile>> {
        let conn = self.get_connection().await?;

        BusinessProfile::list_by_merchant_id(conn.as_async_conn(), merchant_id).await
    }

    async fn insert_business_profile(
        &self,
        business_profile: BusinessProfileNew,
    ) -> StorageResult<BusinessProfile> {
        let conn = self.get_connection().await?;
        business_profile.insert(conn.as_async_conn()).await
    }

    async fn find_business_profile_by_id(
        &self,
        profile_id: &str,
    ) -> StorageResult<BusinessProfile> {
        let conn = self.get_connection().await?;
        BusinessProfile::find_by_profile_id(conn.as_async_conn(), profile_id).await
    }

    async fn update_business_profile(
        &self,
        profile_id: &str,
        business_profile_update: BusinessProfileUpdate,
    ) -> StorageResult<BusinessProfile> {
        let conn = self.get_connection().await?;

        BusinessProfile::update_by_profile_id(
            conn.as_async_conn(),
            profile_id,
            business_profile_update.into(),
        )
        .await
    }
}
