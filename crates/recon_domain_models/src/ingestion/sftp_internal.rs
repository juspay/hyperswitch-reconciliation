use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SftpInternalIngestionConfig {
    pub file_path: PathBuf,
}
