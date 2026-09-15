use actix_web::http::header::HeaderMap;
use base64::{engine::general_purpose, Engine};
use common_utils::request::{Method, RequestBuilder};
use error_stack::{report, ResultExt};

use common_types::ingestion::{
    adyen::{AdyenTriggerData, EventCode, NotificationItemWrapper},
    TriggerData, TriggerDataV1,
};
use external_services::http_client;
use masking::PeekInterface;
use recon_domain_models::ingestion::adyen::{AdyenBasicAuth, AdyenIngestionConfig};
use recon_interfaces::types::Proxy;

use crate::{
    domain::ingestion::FileDownloadResult,
    errors::ingestion::{
        DownloadStageError, DownloadStageResult, ParseStageError, ParseStageResult,
    },
};

/// Adyen-specific ingestion context with all required data
#[derive(Clone, Debug)]
pub struct AdyenIngestionContext {
    pub decrypted_config: AdyenIngestionConfig,
    pub raw_payload: serde_json::Value,
    pub headers: HeaderMap,
}

#[derive(Clone, Debug)]
pub struct AdyenFileDownloadContext {
    pub trigger_data: AdyenTriggerData,
    pub report_basic_auth: AdyenBasicAuth,
}

/// Parse Adyen webhook payload into TriggerData
pub fn parse_adyen_payload(
    raw_payload: serde_json::Value,
    headers: &HeaderMap,
    hmac_secret: &str,
    webhook_basic_auth: &AdyenBasicAuth,
) -> ParseStageResult<TriggerData> {
    // Validate Basic Auth
    validate_basic_auth(headers, webhook_basic_auth)?;

    // Parse the payload into AdyenTriggerData
    let adyen_data = serde_json::from_value::<AdyenTriggerData>(raw_payload).map_err(|e| {
        let reason = e.to_string();
        report!(e).change_context(ParseStageError::JsonParseError { reason })
    })?;

    // Validate HMAC signature
    validate_webhook_signature(&adyen_data, hmac_secret)?;

    Ok(TriggerData::V1(TriggerDataV1::Adyen(adyen_data)))
}

/// Download Adyen report using the trigger data and authentication
pub async fn download_adyen_report(
    context: &AdyenFileDownloadContext,
    proxy_config: &Proxy,
) -> DownloadStageResult<FileDownloadResult> {
    // Validate Adyen-specific requirements
    validate_report_available(&context.trigger_data)?;

    // Get download URL
    let download_url = get_report_download_url(&context.trigger_data)?;

    let file_content =
        download_file(&download_url, &context.report_basic_auth, proxy_config).await?;

    // Get filename from pspReference of the REPORT_AVAILABLE notification
    let file_name = get_report_filename(&context.trigger_data)?;

    Ok(FileDownloadResult {
        file_content,
        file_name,
    })
}

fn validate_basic_auth(
    headers: &HeaderMap,
    webhook_basic_auth: &AdyenBasicAuth,
) -> ParseStageResult<()> {
    let auth_header = headers
        .get("Authorization")
        .or_else(|| headers.get("authorization"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !auth_header.starts_with("Basic ") {
        return Err(ParseStageError::InvalidAuthentication {
            reason: "Missing or invalid Basic Auth header".to_string(),
        }
        .into());
    }

    let encoded = &auth_header[6..]; // Strip "Basic "
    let decoded = general_purpose::STANDARD.decode(encoded).change_context(
        ParseStageError::InvalidAuthentication {
            reason: "Invalid base64 in Basic Auth".to_string(),
        },
    )?;

    let decoded_str =
        String::from_utf8(decoded).change_context(ParseStageError::InvalidAuthentication {
            reason: "Invalid UTF-8 in Basic Auth".to_string(),
        })?;

    let mut parts = decoded_str.splitn(2, ':');
    let user = parts.next().unwrap_or("");
    let pass = parts.next().unwrap_or("");

    if user != webhook_basic_auth.username || pass != webhook_basic_auth.password.get_inner().peek()
    {
        return Err(ParseStageError::InvalidAuthentication {
            reason: "Basic Auth credentials mismatch".to_string(),
        }
        .into());
    }

    Ok(())
}

/// Validate Adyen HMAC signature following Standard webhook format
fn validate_webhook_signature(
    webhook_data: &AdyenTriggerData,
    hmac_secret: &str,
) -> ParseStageResult<()> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    // Step 1: Extract notification item
    let notification_item = webhook_data.notification_items.first().ok_or_else(|| {
        report!(ParseStageError::MissingField {
            field: "notification_items".to_string(),
        })
    })?;

    let notification = &notification_item.notification_request_item;

    // Step 2: Extract signature
    let hmac_signature = if !notification.additional_data.hmac_signature.is_empty() {
        notification.additional_data.hmac_signature.clone()
    } else {
        return Err(ParseStageError::MissingField {
            field: "hmacSignature".to_string(),
        }
        .into());
    };

    // Step 3: Format the payload string
    let original_reference = notification.original_reference.as_deref().unwrap_or("");
    let event_code_str = match notification.event_code {
        EventCode::ReportAvailable => "REPORT_AVAILABLE",
        // Add more as needed
    };

    let payload_string = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        notification.psp_reference,
        original_reference,
        notification.merchant_account_code,
        notification.merchant_reference,
        notification.amount.value,
        notification.amount.currency,
        event_code_str,
        notification.success.to_string().to_lowercase(),
    );

    // Step 4: Decode HMAC secret
    let hmac_key_bytes =
        hex::decode(hmac_secret).change_context(ParseStageError::InvalidFormat {
            reason: "Invalid hexadecimal HMAC secret".to_string(),
        })?;

    // Step 5: Compute signature
    let mut mac = HmacSha256::new_from_slice(&hmac_key_bytes).change_context(
        ParseStageError::InvalidFormat {
            reason: "Invalid HMAC key length".to_string(),
        },
    )?;

    mac.update(payload_string.as_bytes());
    let computed_signature = mac.finalize().into_bytes();
    let computed_base64 = general_purpose::STANDARD.encode(computed_signature);

    // Step 6: Compare
    if hmac_signature != computed_base64 {
        return Err(ParseStageError::InvalidHmacSignature.into());
    }

    Ok(())
}

/// Download file using basic authentication and proxy-aware HTTP client
async fn download_file(
    url: &str,
    basic_auth: &AdyenBasicAuth,
    proxy_config: &Proxy,
) -> DownloadStageResult<Vec<u8>> {
    // Validate URL
    if url.is_empty() || !url.starts_with("http") {
        return Err(DownloadStageError::InvalidUrl {
            url: url.to_string(),
        }
        .into());
    }

    // Prepare basic auth credentials
    let credentials = format!(
        "{}:{}",
        basic_auth.username,
        basic_auth.password.get_inner().peek()
    );
    let encoded = general_purpose::STANDARD.encode(credentials);

    // Build request using the proxy-aware HTTP client
    let authorization_header = format!("Basic {encoded}");
    let request = RequestBuilder::new()
        .method(Method::Get)
        .url(url)
        .header("Accept-Encoding", "gzip")
        .header("User-Agent", "hyperswitch-recon/1.0")
        .header("Authorization", &authorization_header)
        .attach_default_headers()
        .build();

    // Send request using proxy-aware client with 5 minute timeout
    let response = http_client::send_request(proxy_config, request, Some(300))
        .await
        .change_context(DownloadStageError::HttpClientError {
            reason: "Failed to send HTTP request".to_string(),
        })?;

    // Check if request was successful
    if !response.status().is_success() {
        let status_code = response.status().as_u16();
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Err(if status_code == 401 || status_code == 403 {
            DownloadStageError::AuthenticationFailed { reason: message }.into()
        } else {
            DownloadStageError::ExternalServiceError {
                message: format!("HTTP {status_code} - {message}"),
            }
            .into()
        });
    }

    // Download the file content
    let bytes = response
        .bytes()
        .await
        .change_context(DownloadStageError::ResponseReadError)?;

    Ok(bytes.to_vec())
}

/// Validate Adyen webhook data for REPORT_AVAILABLE events
fn validate_report_available(data: &AdyenTriggerData) -> DownloadStageResult<()> {
    if data.notification_items.is_empty() {
        return Err(DownloadStageError::ReportNotAvailable {
            reason: "Empty notification items".to_string(),
        }
        .into());
    }

    if find_report_notification(data).is_none() {
        return Err(DownloadStageError::ReportNotAvailable {
            reason: "No REPORT_AVAILABLE event found in notification".to_string(),
        }
        .into());
    }

    Ok(())
}

/// Get the file download URL for Adyen reports
fn get_report_download_url(data: &AdyenTriggerData) -> DownloadStageResult<String> {
    // Find the first REPORT_AVAILABLE notification
    let report_notification =
        find_report_notification(data).ok_or(report!(DownloadStageError::ReportNotAvailable {
            reason: "No REPORT_AVAILABLE event found in notification".to_string(),
        }))?;

    // Extract the download URL from the reason field
    let download_url = &report_notification.notification_request_item.reason;

    // Validate that the reason field contains a valid URL
    if download_url.is_empty() || !download_url.starts_with("http") {
        return Err(DownloadStageError::InvalidUrl {
            url: download_url.clone(),
        }
        .into());
    }

    Ok(download_url.clone())
}

/// Find the first REPORT_AVAILABLE notification
fn find_report_notification(data: &AdyenTriggerData) -> Option<&NotificationItemWrapper> {
    data.notification_items.iter().find(|item| {
        matches!(
            item.notification_request_item.event_code,
            EventCode::ReportAvailable
        )
    })
}

/// Get the filename for the Adyen report from pspReference
fn get_report_filename(data: &AdyenTriggerData) -> DownloadStageResult<String> {
    find_report_notification(data)
        .map(|item| item.notification_request_item.psp_reference.clone())
        .ok_or(
            DownloadStageError::ReportNotAvailable {
                reason: "No REPORT_AVAILABLE event found".to_string(),
            }
            .into(),
        )
}
