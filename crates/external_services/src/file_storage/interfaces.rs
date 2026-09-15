//! File storage interfaces and error types

use async_trait::async_trait;
use error_stack::Result;

/// Trait for file storage operations
#[async_trait]
pub trait FileStorageInterface: Send + Sync {
    /// Uploads a file to the storage backend
    async fn upload_file(&self, file_key: &str, file_data: Vec<u8>)
        -> Result<(), FileStorageError>;

    /// Retrieves a file from the storage backend
    async fn retrieve_file(&self, file_key: &str) -> Result<Vec<u8>, FileStorageError>;

    /// Deletes a file from the storage backend
    async fn delete_file(&self, file_key: &str) -> Result<(), FileStorageError>;

    /// Checks if a file exists in the storage backend
    async fn file_exists(&self, file_key: &str) -> Result<bool, FileStorageError>;

    /// Generate a presigned URL for file access (optional, not all backends support this)
    async fn generate_presigned_url(
        &self,
        file_key: &str,
        expires_in_secs: u64,
    ) -> Result<Option<String>, FileStorageError> {
        // Default implementation returns None (not supported)
        let _ = (file_key, expires_in_secs);
        Ok(None)
    }
}

/// Errors that can occur during file storage operations
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FileStorageError {
    /// File upload operation failed
    #[error("Failed to upload file")]
    UploadFailed,

    /// File retrieval operation failed
    #[error("Failed to retrieve file")]
    RetrieveFailed,

    /// File deletion operation failed
    #[error("Failed to delete file")]
    DeleteFailed,

    /// File not found
    #[error("File not found")]
    FileNotFound,

    /// Invalid file key or path
    #[error("Invalid file key")]
    InvalidFileKey,

    /// Storage backend configuration error
    #[error("Storage configuration error")]
    ConfigurationError,

    /// Network or connectivity error
    #[error("Network error")]
    NetworkError,

    /// Permission or authentication error
    #[error("Permission denied")]
    PermissionDenied,

    /// Storage quota exceeded
    #[error("Storage quota exceeded")]
    QuotaExceeded,

    /// Generic storage error
    #[error("Storage error: {message}")]
    StorageError { message: String },
}
