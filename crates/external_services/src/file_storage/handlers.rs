//! File storage handler implementation

use std::sync::Arc;

use error_stack::{Result, ResultExt};

use super::{
    config::FileStorageConfig,
    implementations::{AwsS3Storage, FileSystemStorage, OciObjectStorage},
    interfaces::{FileStorageError, FileStorageInterface},
};

/// Handler for file storage operations
pub struct FileStorageHandler {
    storage: Arc<dyn FileStorageInterface>,
}

impl FileStorageHandler {
    /// Creates a new file storage handler from configuration
    pub async fn from_config(config: &FileStorageConfig) -> Result<Self, FileStorageError> {
        // Validate configuration first
        if let Err(validation_error) = config.validate() {
            return Err(FileStorageError::ConfigurationError).attach_printable(validation_error);
        }

        let storage: Arc<dyn FileStorageInterface> = match config {
            FileStorageConfig::AwsS3 { aws_s3 } => Arc::new(AwsS3Storage::new(aws_s3).await?),
            FileStorageConfig::OciObjectStorage { oci_object_storage } => {
                Arc::new(OciObjectStorage::new(oci_object_storage).await?)
            }
            FileStorageConfig::FileSystem { file_system } => {
                Arc::new(FileSystemStorage::new(file_system)?)
            }
        };

        Ok(Self { storage })
    }

    /// Uploads a file to the configured storage backend
    pub async fn upload_file(
        &self,
        file_key: &str,
        file_data: Vec<u8>,
    ) -> Result<(), FileStorageError> {
        self.storage.upload_file(file_key, file_data).await
    }

    /// Retrieves a file from the configured storage backend
    pub async fn retrieve_file(&self, file_key: &str) -> Result<Vec<u8>, FileStorageError> {
        self.storage.retrieve_file(file_key).await
    }

    /// Deletes a file from the configured storage backend
    pub async fn delete_file(&self, file_key: &str) -> Result<(), FileStorageError> {
        self.storage.delete_file(file_key).await
    }

    /// Checks if a file exists in the configured storage backend
    pub async fn file_exists(&self, file_key: &str) -> Result<bool, FileStorageError> {
        self.storage.file_exists(file_key).await
    }

    /// Gets a reference to the underlying storage interface
    pub fn storage(&self) -> &Arc<dyn FileStorageInterface> {
        &self.storage
    }

    /// Generate a presigned URL for file access
    pub async fn generate_presigned_url(
        &self,
        file_key: &str,
        expires_in_secs: u64,
    ) -> Result<Option<String>, FileStorageError> {
        self.storage
            .generate_presigned_url(file_key, expires_in_secs)
            .await
    }
}

impl Clone for FileStorageHandler {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
        }
    }
}
