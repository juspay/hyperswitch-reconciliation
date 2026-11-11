use api_models::api_key::{ApiKeyResponse, CreateApiKeyRequest, UpdateApiKeyRequest};
use diesel_models::api_key::ApiKey;
use error_stack::Report;
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
        return Err(Report::new(ApiKeyError::NameEmpty));
    }
    if name.chars().count() > MAX_NAME_LENGTH {
        return Err(Report::new(ApiKeyError::NameTooLong {
            max_length: MAX_NAME_LENGTH,
        }));
    }
    Ok(())
}

fn validate_description(description: &str) -> ApiKeyResult<()> {
    if description.chars().count() > MAX_DESCRIPTION_LENGTH {
        return Err(Report::new(ApiKeyError::DescriptionTooLong {
            max_length: MAX_DESCRIPTION_LENGTH,
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
            return Err(Report::new(ApiKeyError::ExpiryInPast));
        }
        if self.expires_at > now + Duration::days(MAX_EXPIRY_DAYS) {
            return Err(Report::new(ApiKeyError::ExpiryTooFar {
                max_days: MAX_EXPIRY_DAYS,
            }));
        }
        Ok(())
    }
}

impl ApiKeyRequestValidator for UpdateApiKeyRequest {
    fn validate(&self, _now: PrimitiveDateTime) -> ApiKeyResult<()> {
        let details = match self {
            Self::RevokeUpdate(_) => return Ok(()),
            Self::DetailsUpdate(details) => details,
        };
        if details.name.is_none() && details.description.is_none() {
            return Err(Report::new(ApiKeyError::NothingToUpdate));
        }
        if let Some(name) = &details.name {
            validate_name(name)?;
        }
        if let Some(description) = &details.description {
            validate_description(description)?;
        }
        Ok(())
    }
}
