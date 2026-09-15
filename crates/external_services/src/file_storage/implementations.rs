//! File storage implementations

pub mod aws_s3;
pub mod file_system;

// Re-export implementations
pub use aws_s3::AwsS3Storage;
pub use file_system::FileSystemStorage;
