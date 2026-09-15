#[derive(Clone, Debug)]
pub struct ManualFileDownloadContext {
    pub file_data: Vec<u8>,
    pub file_name: String,
}
