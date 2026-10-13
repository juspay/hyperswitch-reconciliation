//! OCI Object Storage via its S3-compatible API.

use std::time::Duration;

use async_trait::async_trait;
use aws_config::{meta::region::RegionProviderChain, Region};
use aws_sdk_s3::{
    config::{BehaviorVersion, Builder as S3ConfigBuilder},
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client as S3Client,
};
use error_stack::{Report, Result, ResultExt};

use crate::file_storage::{
    config::OciObjectStorageConfig,
    interfaces::{FileStorageError, FileStorageInterface},
};

#[derive(Debug, Clone)]
pub struct OciObjectStorage {
    client: S3Client,
    bucket_name: String,
}

impl OciObjectStorage {
    pub async fn new(config: &OciObjectStorageConfig) -> Result<Self, FileStorageError> {
        config.validate().map_err(|validation_error| {
            Report::new(FileStorageError::ConfigurationError).attach_printable(validation_error)
        })?;

        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        // Deployment supplies OCI customer-secret keys through the SDK credential chain.
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        let s3_config_builder = S3ConfigBuilder::from(&aws_config)
            .endpoint_url(&config.endpoint_url)
            .force_path_style(config.force_path_style);

        let s3_config = s3_config_builder.build();
        let client = S3Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket_name: config.bucket_name.clone(),
        })
    }

    fn validate_file_key(&self, file_key: &str) -> Result<(), FileStorageError> {
        if file_key.is_empty() {
            return Err(FileStorageError::InvalidFileKey)
                .attach_printable("File key cannot be empty");
        }

        if file_key.starts_with('/') {
            return Err(FileStorageError::InvalidFileKey)
                .attach_printable("File key should not start with '/'");
        }

        if file_key.contains('\0') {
            return Err(FileStorageError::InvalidFileKey)
                .attach_printable("File key contains null character");
        }

        Ok(())
    }
}

#[async_trait]
impl FileStorageInterface for OciObjectStorage {
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
            .attach_printable("Failed to upload file to OCI Object Storage")?;

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
            .attach_printable("Failed to retrieve file from OCI Object Storage")?;

        let data = response
            .body
            .collect()
            .await
            .change_context(FileStorageError::RetrieveFailed)
            .attach_printable("Failed to read OCI object body")?
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
            .attach_printable("Failed to delete file from OCI Object Storage")?;

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
                if let Some(service_err) = err.as_service_error() {
                    if service_err.is_not_found() {
                        return Ok(false);
                    }
                }

                Err(err)
                    .change_context(FileStorageError::RetrieveFailed)
                    .attach_printable("Failed to check if file exists in OCI Object Storage")
            }
        }
    }

    async fn generate_presigned_url(
        &self,
        file_key: &str,
        expires_in_secs: u64,
    ) -> Result<Option<String>, FileStorageError> {
        self.validate_file_key(file_key)?;

        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(expires_in_secs))
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
            .attach_printable("Failed to generate presigned URL for OCI Object Storage")?;

        Ok(Some(presigned_request.uri().to_string()))
    }
}
