use api_models::api_key::{ApiKeyResponse, CreateApiKeyRequest, UpdateApiKeyRequest};
use common_enums::ApiKeyStatus;
use diesel_models::api_key::ApiKey;
use error_stack::report;
use time::{Duration, PrimitiveDateTime};

use crate::errors::api_key::{ApiKeyError, ApiKeyResult};

pub trait ApiKeyExt {
    fn to_api_response(self) -> ApiKeyResponse;
}

impl ApiKeyExt for ApiKey {
    fn to_api_response(self) -> ApiKeyResponse {
        ApiKeyResponse {
            id: self.id,
            merchant_id: self.merchant_id,
            name: self.name,
            description: self.description,
            prefix: self.prefix,
            created_by: self.created_by,
            status: self.status,
            created_at: self.created_at,
            expires_at: self.expires_at,
            last_modified_at: self.last_modified_at,
        }
    }
}

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_DESCRIPTION_LENGTH: usize = 256;
pub const MAX_EXPIRY_DAYS: i64 = 365;

pub trait ApiKeyRequestValidator {
    /// `now` is passed in so the caller can use the same instant for `created_at`.
    fn validate(&self, now: PrimitiveDateTime) -> ApiKeyResult<()>;
}

fn validate_name(name: &str) -> ApiKeyResult<()> {
    if name.trim().is_empty() {
        return Err(report!(ApiKeyError::NameEmpty));
    }
    if name.chars().count() > MAX_NAME_LENGTH {
        return Err(report!(ApiKeyError::NameTooLong {
            max_length: MAX_NAME_LENGTH
        }));
    }
    Ok(())
}

fn validate_description(description: &str) -> ApiKeyResult<()> {
    if description.chars().count() > MAX_DESCRIPTION_LENGTH {
        return Err(report!(ApiKeyError::DescriptionTooLong {
            max_length: MAX_DESCRIPTION_LENGTH
        }));
    }
    Ok(())
}

impl ApiKeyRequestValidator for CreateApiKeyRequest {
    fn validate(&self, now: PrimitiveDateTime) -> ApiKeyResult<()> {
        validate_name(&self.name)?;
        if let Some(description) = &self.description {
            validate_description(description)?;
        }
        if self.expires_at <= now {
            return Err(report!(ApiKeyError::ExpiryInPast));
        }
        if self.expires_at > now + Duration::days(MAX_EXPIRY_DAYS) {
            return Err(report!(ApiKeyError::ExpiryTooFar {
                max_days: MAX_EXPIRY_DAYS
            }));
        }
        Ok(())
    }
}

impl ApiKeyRequestValidator for UpdateApiKeyRequest {
    fn validate(&self, _now: PrimitiveDateTime) -> ApiKeyResult<()> {
        if self.name.is_none() && self.description.is_none() && self.status.is_none() {
            return Err(report!(ApiKeyError::NothingToUpdate));
        }
        if let Some(name) = &self.name {
            validate_name(name)?;
        }
        if let Some(description) = &self.description {
            validate_description(description)?;
        }
        if let Some(status) = self.status {
            if status != ApiKeyStatus::Revoked {
                return Err(report!(ApiKeyError::InvalidStatusUpdate));
            }
            if self.name.is_some() || self.description.is_some() {
                return Err(report!(ApiKeyError::StatusUpdateWithDetails));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::macros::datetime;

    use super::*;

    const NOW: PrimitiveDateTime = datetime!(2026-01-01 00:00);

    fn create_request(name: &str, expires_at: PrimitiveDateTime) -> CreateApiKeyRequest {
        CreateApiKeyRequest {
            name: name.to_string(),
            description: None,
            expires_at,
        }
    }

    fn update_request() -> UpdateApiKeyRequest {
        UpdateApiKeyRequest {
            name: None,
            description: None,
            status: None,
        }
    }

    fn err_of(result: ApiKeyResult<()>) -> ApiKeyError {
        result.unwrap_err().current_context().clone()
    }

    #[test]
    fn create_accepts_valid_request() {
        assert!(create_request("key", NOW + Duration::days(30))
            .validate(NOW)
            .is_ok());
    }

    #[test]
    fn create_rejects_blank_and_long_names() {
        let expiry = NOW + Duration::days(1);
        assert!(matches!(
            err_of(create_request("   ", expiry).validate(NOW)),
            ApiKeyError::NameEmpty
        ));
        assert!(matches!(
            err_of(create_request(&"a".repeat(65), expiry).validate(NOW)),
            ApiKeyError::NameTooLong { .. }
        ));
        assert!(create_request(&"é".repeat(64), expiry)
            .validate(NOW)
            .is_ok());
    }

    #[test]
    fn create_rejects_long_description() {
        let mut request = create_request("key", NOW + Duration::days(1));
        request.description = Some("a".repeat(257));
        assert!(matches!(
            err_of(request.validate(NOW)),
            ApiKeyError::DescriptionTooLong { .. }
        ));
    }

    #[test]
    fn create_checks_expiry_bounds() {
        assert!(matches!(
            err_of(create_request("key", NOW).validate(NOW)),
            ApiKeyError::ExpiryInPast
        ));
        assert!(create_request("key", NOW + Duration::days(365))
            .validate(NOW)
            .is_ok());
        assert!(matches!(
            err_of(
                create_request("key", NOW + Duration::days(365) + Duration::seconds(1))
                    .validate(NOW)
            ),
            ApiKeyError::ExpiryTooFar { .. }
        ));
    }

    #[test]
    fn update_rejects_empty_request() {
        assert!(matches!(
            err_of(update_request().validate(NOW)),
            ApiKeyError::NothingToUpdate
        ));
    }

    #[test]
    fn update_only_allows_revoked_status() {
        let mut request = update_request();
        request.status = Some(ApiKeyStatus::Active);
        assert!(matches!(
            err_of(request.validate(NOW)),
            ApiKeyError::InvalidStatusUpdate
        ));
        request.status = Some(ApiKeyStatus::Revoked);
        assert!(request.validate(NOW).is_ok());
    }

    #[test]
    fn update_rejects_status_together_with_details() {
        let mut request = update_request();
        request.status = Some(ApiKeyStatus::Revoked);
        request.name = Some("new".to_string());
        assert!(matches!(
            err_of(request.validate(NOW)),
            ApiKeyError::StatusUpdateWithDetails
        ));

        let mut request = update_request();
        request.status = Some(ApiKeyStatus::Revoked);
        request.description = Some("new".to_string());
        assert!(matches!(
            err_of(request.validate(NOW)),
            ApiKeyError::StatusUpdateWithDetails
        ));
    }

    #[test]
    fn update_validates_name_and_description() {
        let mut request = update_request();
        request.name = Some(" ".to_string());
        assert!(matches!(
            err_of(request.validate(NOW)),
            ApiKeyError::NameEmpty
        ));
        let mut request = update_request();
        request.description = Some("a".repeat(257));
        assert!(matches!(
            err_of(request.validate(NOW)),
            ApiKeyError::DescriptionTooLong { .. }
        ));
    }
}
