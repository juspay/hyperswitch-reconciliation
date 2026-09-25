use api_models::business_profile as api;
use recon_domain_models::business_profile::DomainBusinessProfile;

pub trait BusinessProfileExt {
    fn to_api_response(self) -> api::BusinessProfileResponse;
}

impl BusinessProfileExt for DomainBusinessProfile {
    fn to_api_response(self) -> api::BusinessProfileResponse {
        api::BusinessProfileResponse {
            profile_id: self.profile_id,
            merchant_id: self.merchant_id,
            profile_name: self.profile_name,
            timezone: self.timezone,
        }
    }
}
