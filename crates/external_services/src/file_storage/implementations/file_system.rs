//! Local file system storage implementation

use std::{
    fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use tokio::fs as async_fs;

use crate::file_storage::{
    config::FileSystemConfig,
    interfaces::{FileStorageError, FileStorageInterface},
};

/// Local file system storage implementation
#[derive(Debug, Clone)]
pub struct FileSystemStorage {
    base_path: PathBuf,
    create_directories: bool,
}

impl FileSystemStorage {
    /// Creates a new file system storage instance
    pub fn new(config: &FileSystemConfig) -> Result<Self, FileStorageError> {
        let base_path = PathBuf::from(&config.base_path);

        // Create base directory if it doesn't exist and create_directories is true
        if config.create_directories && !base_path.exists() {
            fs::create_dir_all(&base_path)
                .change_context(FileStorageError::ConfigurationError)
                .attach_printable_lazy(|| {
                    format!("Failed to create base directory: {}", base_path.display())
                })?;
        }

        // Verify base path exists and is a directory
        if !base_path.exists() {
            return Err(FileStorageError::ConfigurationError).attach_printable_lazy(|| {
                format!("Base path does not exist: {}", base_path.display())
            });
        }

        if !base_path.is_dir() {
            return Err(FileStorageError::ConfigurationError).attach_printable_lazy(|| {
                format!("Base path is not a directory: {}", base_path.display())
            });
        }

        Ok(Self {
            base_path,
            create_directories: config.create_directories,
        })
    }

    /// Converts a file key to a full file path
    fn get_file_path(&self, file_key: &str) -> Result<PathBuf, FileStorageError> {
        // Validate file key
        if file_key.is_empty() {
            return Err(FileStorageError::InvalidFileKey)
                .attach_printable("File key cannot be empty");
        }

        // Prevent directory traversal attacks
        if file_key.contains("..") || file_key.starts_with('/') {
            return Err(FileStorageError::InvalidFileKey)
                .attach_printable("File key contains invalid characters");
        }

        let file_path = self.base_path.join(file_key);

        // Ensure the resolved path is still within the base directory
        let canonical_base = self
            .base_path
            .canonicalize()
            .change_context(FileStorageError::ConfigurationError)?;

        if let Ok(canonical_file) = file_path.canonicalize() {
            if !canonical_file.starts_with(&canonical_base) {
                return Err(FileStorageError::InvalidFileKey)
                    .attach_printable("File path is outside base directory");
            }
        }

        Ok(file_path)
    }

    /// Creates parent directories for a file path if needed
    async fn ensure_parent_directory(&self, file_path: &Path) -> Result<(), FileStorageError> {
        if let Some(parent) = file_path.parent() {
            if self.create_directories && !parent.exists() {
                async_fs::create_dir_all(parent)
                    .await
                    .change_context(FileStorageError::UploadFailed)
                    .attach_printable_lazy(|| {
                        format!("Failed to create parent directory: {}", parent.display())
                    })?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl FileStorageInterface for FileSystemStorage {
    async fn upload_file(
        &self,
        file_key: &str,
        file_data: Vec<u8>,
    ) -> Result<(), FileStorageError> {
        let file_path = self.get_file_path(file_key)?;

        // Ensure parent directory exists
        self.ensure_parent_directory(&file_path).await?;

        // Write file data
        async_fs::write(&file_path, file_data)
            .await
            .change_context(FileStorageError::UploadFailed)
            .attach_printable_lazy(|| format!("Failed to write file: {}", file_path.display()))?;

        Ok(())
    }

    async fn retrieve_file(&self, file_key: &str) -> Result<Vec<u8>, FileStorageError> {
        let file_path = self.get_file_path(file_key)?;

        if !file_path.exists() {
            return Err(FileStorageError::FileNotFound)
                .attach_printable_lazy(|| format!("File not found: {}", file_path.display()));
        }

        async_fs::read(&file_path)
            .await
            .change_context(FileStorageError::RetrieveFailed)
            .attach_printable_lazy(|| format!("Failed to read file: {}", file_path.display()))
    }

    async fn delete_file(&self, file_key: &str) -> Result<(), FileStorageError> {
        let file_path = self.get_file_path(file_key)?;

        if !file_path.exists() {
            return Err(FileStorageError::FileNotFound)
                .attach_printable_lazy(|| format!("File not found: {}", file_path.display()));
        }

        async_fs::remove_file(&file_path)
            .await
            .change_context(FileStorageError::DeleteFailed)
            .attach_printable_lazy(|| format!("Failed to delete file: {}", file_path.display()))?;

        Ok(())
    }

    async fn file_exists(&self, file_key: &str) -> Result<bool, FileStorageError> {
        let file_path = self.get_file_path(file_key)?;
        Ok(file_path.exists() && file_path.is_file())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(base_path: &str) -> FileSystemConfig {
        FileSystemConfig {
            base_path: base_path.to_string(),
            create_directories: true,
        }
    }

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = create_test_config(temp_dir.path().to_str().unwrap());
        let storage = FileSystemStorage::new(&config).expect("Failed to create storage");

        let file_key = "test/file.txt";
        let file_data = b"Hello, World!".to_vec();

        // Test file doesn't exist initially
        assert!(!storage.file_exists(file_key).await.unwrap());

        // Test upload
        storage
            .upload_file(file_key, file_data.clone())
            .await
            .expect("Failed to upload file");

        // Test file exists after upload
        assert!(storage.file_exists(file_key).await.unwrap());

        // Test retrieve
        let retrieved_data = storage
            .retrieve_file(file_key)
            .await
            .expect("Failed to retrieve file");
        assert_eq!(retrieved_data, file_data);

        // Test delete
        storage
            .delete_file(file_key)
            .await
            .expect("Failed to delete file");

        // Test file doesn't exist after delete
        assert!(!storage.file_exists(file_key).await.unwrap());
    }

    #[tokio::test]
    async fn test_invalid_file_key() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = create_test_config(temp_dir.path().to_str().unwrap());
        let storage = FileSystemStorage::new(&config).expect("Failed to create storage");

        // Test directory traversal protection
        let result = storage.upload_file("../test.txt", b"data".to_vec()).await;
        assert!(result.is_err());

        // Test absolute path protection
        let result = storage.upload_file("/etc/passwd", b"data".to_vec()).await;
        assert!(result.is_err());

        // Test empty file key
        let result = storage.upload_file("", b"data".to_vec()).await;
        assert!(result.is_err());
    }
}
