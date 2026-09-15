use std::path::PathBuf;

use api_models::{
    ingestion::{
        IngestionFileUploadRequest, IngestionHistoryResponse, IngestionUploadType,
        ProcessedIngestionFileRequest,
    },
    transformation::TransformationTriggerRequest,
};
use common_types::ingestion::{IngestionData, IngestionDataV1};

use diesel_models::ingestion_config::{IngestionConfigData, IngestionConfigPsp, IngestionConfigV1};

use common_utils::{errors::ReportSwitchExt, logging};
use diesel_models::ingestion_history::IngestionConfigHistoryView;

use crate::{
    errors::ingestion::{IngestionResult, ParseStageError},
    AppState,
};

pub fn process_multipart_request(
    upload_request: IngestionFileUploadRequest,
) -> IngestionResult<ProcessedIngestionFileRequest> {
    let file_data = upload_request.file.data;

    // Get filename - required field
    let file_name = upload_request
        .file
        .file_name
        .ok_or(
            ParseStageError::MissingField {
                field: "file_name".to_string(),
            }
            .into(),
        )
        .switch()?;

    Ok(ProcessedIngestionFileRequest {
        file_data: file_data.to_vec(),
        file_name,
    })
}

/// Notify transformation service to process the ingested file via HTTP API call
pub async fn notify_transformation_service(
    state: &AppState,
    ingestion_id: &str,
    ingestion_history_id: &str,
    file_path: PathBuf,
) {
    let trigger_request = TransformationTriggerRequest { file_path };

    // Make HTTP API call to transformation service endpoint (same server)
    let client = reqwest::Client::new();
    let transformation_url = format!(
        "{}/transformations/{}/{}/trigger",
        state.ingestion.config.transformation_base_url, ingestion_id, ingestion_history_id
    );

    // No authentication required - transformation trigger uses NoAuth like ingestion trigger
    let result = client
        .post(&transformation_url)
        .json(&trigger_request)
        .send()
        .await;

    // Log the result for debugging (but don't fail the ingestion)
    match result {
        Ok(response) => {
            if response.status().is_success() {
                logging::info!(
                    "Transformation triggered successfully for ingestion {}, history {}",
                    ingestion_id,
                    ingestion_history_id
                );
            } else {
                logging::warn!(
                    "Failed to trigger transformation for ingestion {}, history {}: HTTP {}",
                    ingestion_id,
                    ingestion_history_id,
                    response.status()
                );
            }
        }
        Err(e) => {
            logging::error!(
                "Failed to trigger transformation for ingestion {}, history {}: {}",
                ingestion_id,
                ingestion_history_id,
                e
            );
        }
    }
}

pub trait IngestionDataExt {
    fn get_file_name(&self) -> Option<String>;
    fn get_file_key(&self) -> Option<&PathBuf>;
}

impl IngestionDataExt for IngestionData {
    fn get_file_name(&self) -> Option<String> {
        match self {
            Self::V1(IngestionDataV1::Processed(data)) => data
                .file_key
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string()),
            Self::V1(IngestionDataV1::Failed(_)) => None,
        }
    }

    fn get_file_key(&self) -> Option<&PathBuf> {
        match self {
            Self::V1(IngestionDataV1::Processed(data)) => Some(&data.file_key),
            Self::V1(IngestionDataV1::Failed(_)) => None,
        }
    }
}

pub trait IngestionConfigHistoryViewExt {
    fn to_api_response(self) -> IngestionHistoryResponse;
}

impl IngestionConfigHistoryViewExt for IngestionConfigHistoryView {
    fn to_api_response(self) -> IngestionHistoryResponse {
        let file_name = self.history.data.and_then(|data| data.get_file_name());

        IngestionHistoryResponse {
            id: self.history.id,
            ingestion_id: self.history.ingestion_config_id,
            ingestion_history_id: self.history.ingestion_history_id,
            file_name,
            account_id: self.config.account_id,
            status: self.history.status,
            upload_type: self.config.config.get_upload_type(),
            created_at: self.history.created_at,
            ingestion_name: self.config.name,
            version: self.history.version,
            discarded_at: self.history.discarded_at,
            discarded_status: self.history.discarded_status,
        }
    }
}

pub trait IngestionConfigDataApiExt {
    fn get_upload_type(&self) -> IngestionUploadType;
}

impl IngestionConfigDataApiExt for IngestionConfigData {
    fn get_upload_type(&self) -> IngestionUploadType {
        match self {
            Self::V1(IngestionConfigV1::Psp(IngestionConfigPsp::Adyen(_))) => {
                IngestionUploadType::Adyen
            }
            Self::V1(IngestionConfigV1::Manual) => IngestionUploadType::Manual,
            Self::V1(IngestionConfigV1::SftpInternal(_)) => IngestionUploadType::SftpInternal,
        }
    }
}

pub fn generate_file_key(ingestion_id: &str, history_id: &str, file_name: &str) -> PathBuf {
    ["ingestions", ingestion_id, history_id, file_name]
        .iter()
        .collect()
}
