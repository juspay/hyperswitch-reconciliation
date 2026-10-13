//! File storage implementations

pub mod aws_s3;
pub mod file_system;
pub mod oci_object_storage;

// Re-export implementations
pub use aws_s3::AwsS3Storage;
pub use file_system::FileSystemStorage;
pub use oci_object_storage::OciObjectStorage;
