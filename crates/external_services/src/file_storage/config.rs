//! File storage configuration

use serde::Deserialize;
use url::Url;

/// Configuration for file storage backends
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "file_storage_backend")]
#[serde(rename_all = "snake_case")]
pub enum FileStorageConfig {
    /// AWS S3 storage configuration
    AwsS3 {
        /// AWS S3 specific configuration
        aws_s3: AwsS3Config,
    },
    /// OCI Object Storage using its S3-compatible API
    OciObjectStorage {
        oci_object_storage: OciObjectStorageConfig,
    },
    /// Local file system storage configuration
    FileSystem {
        /// Local file system specific configuration
        file_system: FileSystemConfig,
    },
}

impl Default for FileStorageConfig {
    fn default() -> Self {
        Self::FileSystem {
            file_system: FileSystemConfig::default(),
        }
    }
}

/// AWS S3 storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AwsS3Config {
    /// AWS region for S3 bucket
    pub region: String,
    /// S3 bucket name
    pub bucket_name: String,
}

impl AwsS3Config {
    /// Validates the AWS S3 configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.region.trim().is_empty() {
            return Err("AWS S3 region cannot be empty".to_string());
        }
        if self.bucket_name.trim().is_empty() {
            return Err("AWS S3 bucket name cannot be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OciObjectStorageConfig {
    pub region: String,
    pub bucket_name: String,
    pub endpoint_url: String,
    pub force_path_style: bool,
}

impl OciObjectStorageConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.region.trim().is_empty() {
            return Err("OCI Object Storage region cannot be empty".to_string());
        }
        if self.bucket_name.trim().is_empty() {
            return Err("OCI Object Storage bucket name cannot be empty".to_string());
        }
        let endpoint = Url::parse(&self.endpoint_url)
            .map_err(|_| "OCI Object Storage endpoint must be a valid HTTPS URL".to_string())?;
        if endpoint.scheme() != "https" {
            return Err("OCI Object Storage endpoint must use HTTPS".to_string());
        }
        if !endpoint.username().is_empty() || endpoint.password().is_some() {
            return Err("OCI Object Storage endpoint cannot contain credentials".to_string());
        }
        Ok(())
    }
}

/// Local file system storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct FileSystemConfig {
    /// Base directory for file storage
    pub base_path: String,
    /// Whether to create directories if they don't exist
    pub create_directories: bool,
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        Self {
            base_path: "./storage".to_string(),
            create_directories: true,
        }
    }
}

impl FileSystemConfig {
    /// Validates the file system configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.base_path.trim().is_empty() {
            return Err("File system base path cannot be empty".to_string());
        }
        Ok(())
    }
}

impl FileStorageConfig {
    /// Validates the file storage configuration
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::AwsS3 { aws_s3 } => aws_s3.validate(),
            Self::OciObjectStorage { oci_object_storage } => oci_object_storage.validate(),
            Self::FileSystem { file_system } => file_system.validate(),
        }
    }
}
