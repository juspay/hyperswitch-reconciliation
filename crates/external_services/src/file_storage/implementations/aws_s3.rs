//! AWS S3 storage implementation

use async_trait::async_trait;
use aws_config::{meta::region::RegionProviderChain, Region};
use aws_sdk_s3::{config::Builder as S3ConfigBuilder, primitives::ByteStream, Client as S3Client};
use error_stack::{Result, ResultExt};

use crate::file_storage::{
    config::AwsS3Config,
    interfaces::{FileStorageError, FileStorageInterface},
};

/// AWS S3 storage implementation
#[derive(Debug, Clone)]
pub struct AwsS3Storage {
    client: S3Client,
    bucket_name: String,
}

impl AwsS3Storage {
    /// Creates a new AWS S3 storage instance
    pub async fn new(config: &AwsS3Config) -> Result<Self, FileStorageError> {
        // Validate configuration
        if let Err(validation_error) = config.validate() {
            return Err(FileStorageError::ConfigurationError).attach_printable(validation_error);
        }

        // Set up AWS region
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        // Build S3 config
        let s3_config_builder = S3ConfigBuilder::from(&aws_config);

        let s3_config = s3_config_builder.build();
        let client = S3Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket_name: config.bucket_name.clone(),
        })
    }

    /// Validates that the file key is safe for S3
    fn validate_file_key(&self, file_key: &str) -> Result<(), FileStorageError> {
        if file_key.is_empty() {
            return Err(FileStorageError::InvalidFileKey)
                .attach_printable("File key cannot be empty");
        }

        // S3 object keys should not start with '/'
        if file_key.starts_with('/') {
            return Err(FileStorageError::InvalidFileKey)
                .attach_printable("File key should not start with '/'");
        }

        // Check for invalid characters (basic validation)
        if file_key.contains('\0') {
            return Err(FileStorageError::InvalidFileKey)
                .attach_printable("File key contains null character");
        }

        Ok(())
    }

    /// Generate a presigned URL for file access
    pub async fn generate_presigned_url(
        &self,
        file_key: &str,
        expires_in_secs: u64,
    ) -> Result<String, FileStorageError> {
        self.validate_file_key(file_key)?;

        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(
            std::time::Duration::from_secs(expires_in_secs),
        )
        .map_err(|_| FileStorageError::ConfigurationError)
        .attach_printable("Invalid expiration time for presigned URL")?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .presigned(presigning_config)
            .await
            .change_context(FileStorageError::RetrieveFailed)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to generate presigned URL for S3: bucket={}, key={}",
                    self.bucket_name, file_key
                )
            })?;

        Ok(presigned_request.uri().to_string())
    }
}

#[async_trait]
impl FileStorageInterface for AwsS3Storage {
    async fn upload_file(
        &self,
        file_key: &str,
        file_data: Vec<u8>,
    ) -> Result<(), FileStorageError> {
        self.validate_file_key(file_key)?;

        let body = ByteStream::from(file_data);

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .body(body)
            .send()
            .await
            .change_context(FileStorageError::UploadFailed)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to upload file to S3: bucket={}, key={}",
                    self.bucket_name, file_key
                )
            })?;

        Ok(())
    }

    async fn retrieve_file(&self, file_key: &str) -> Result<Vec<u8>, FileStorageError> {
        self.validate_file_key(file_key)?;

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .send()
            .await
            .change_context(FileStorageError::RetrieveFailed)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to retrieve file from S3: bucket={}, key={}",
                    self.bucket_name, file_key
                )
            })?;

        let data = response
            .body
            .collect()
            .await
            .change_context(FileStorageError::RetrieveFailed)
            .attach_printable("Failed to read S3 object body")?
            .into_bytes()
            .to_vec();

        Ok(data)
    }

    async fn delete_file(&self, file_key: &str) -> Result<(), FileStorageError> {
        self.validate_file_key(file_key)?;

        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .send()
            .await
            .change_context(FileStorageError::DeleteFailed)
            .attach_printable_lazy(|| {
                format!(
                    "Failed to delete file from S3: bucket={}, key={}",
                    self.bucket_name, file_key
                )
            })?;

        Ok(())
    }

    async fn file_exists(&self, file_key: &str) -> Result<bool, FileStorageError> {
        self.validate_file_key(file_key)?;

        match self
            .client
            .head_object()
            .bucket(&self.bucket_name)
            .key(file_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(err) => {
                // Check if it's a "not found" error
                if let Some(service_err) = err.as_service_error() {
                    if service_err.is_not_found() {
                        return Ok(false);
                    }
                }
                // For other errors, propagate them
                Err(err)
                    .change_context(FileStorageError::RetrieveFailed)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to check if file exists in S3: bucket={}, key={}",
                            self.bucket_name, file_key
                        )
                    })
            }
        }
    }

    async fn generate_presigned_url(
        &self,
        file_key: &str,
        expires_in_secs: u64,
    ) -> Result<Option<String>, FileStorageError> {
        let url = self
            .generate_presigned_url(file_key, expires_in_secs)
            .await?;
        Ok(Some(url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AwsS3Config {
        AwsS3Config {
            region: "us-east-1".to_string(),
            bucket_name: "test-bucket".to_string(),
        }
    }

    #[test]
    fn test_config_validation() {
        // Test valid config
        let config = create_test_config();
        assert!(config.validate().is_ok());

        // Test empty region
        let mut config = create_test_config();
        config.region = "".to_string();
        assert!(config.validate().is_err());

        // Test empty bucket name
        let mut config = create_test_config();
        config.bucket_name = "".to_string();
        assert!(config.validate().is_err());
    }
}
