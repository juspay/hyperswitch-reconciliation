use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SftpInternalIngestionConfig {
    pub file_path: PathBuf,
}
