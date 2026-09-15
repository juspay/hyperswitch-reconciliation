use common_utils::errors::{ApiClientError, ErrorSwitch};

// http client errors
#[allow(missing_docs, missing_debug_implementations)]
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum HttpClientError {
    #[error("Header map construction failed")]
    HeaderMapConstructionFailed,
    #[error("Invalid proxy configuration")]
    InvalidProxyConfiguration,
    #[error("Client construction failed")]
    ClientConstructionFailed,
    #[error("Certificate decode failed")]
    CertificateDecodeFailed,
    #[error("Request body serialization failed")]
    BodySerializationFailed,
    #[error("Unexpected state reached/Invariants conflicted")]
    UnexpectedState,
    #[error("Failed to parse URL")]
    UrlParsingFailed,
    #[error("URL encoding of request payload failed")]
    UrlEncodingFailed,
    #[error("Failed to send request to connector {0}")]
    RequestNotSent(String),
    #[error("Failed to decode response")]
    ResponseDecodingFailed,
    #[error("Server responded with Request Timeout")]
    RequestTimeoutReceived,
    #[error("connection closed before a message could complete")]
    ConnectionClosedIncompleteMessage,
    #[error("Server responded with Internal Server Error")]
    InternalServerErrorReceived,
    #[error("Server responded with Bad Gateway")]
    BadGatewayReceived,
    #[error("Server responded with Service Unavailable")]
    ServiceUnavailableReceived,
    #[error("Server responded with Gateway Timeout")]
    GatewayTimeoutReceived,
    #[error("Server responded with unexpected response")]
    UnexpectedServerResponse,
}

impl ErrorSwitch<ApiClientError> for HttpClientError {
    fn switch(&self) -> ApiClientError {
        match self {
            Self::HeaderMapConstructionFailed => ApiClientError::HeaderMapConstructionFailed,
            Self::InvalidProxyConfiguration => ApiClientError::InvalidProxyConfiguration,
            Self::ClientConstructionFailed => ApiClientError::ClientConstructionFailed,
            Self::CertificateDecodeFailed => ApiClientError::CertificateDecodeFailed,
            Self::BodySerializationFailed => ApiClientError::BodySerializationFailed,
            Self::UnexpectedState => ApiClientError::UnexpectedState,
            Self::UrlParsingFailed => ApiClientError::UrlParsingFailed,
            Self::UrlEncodingFailed => ApiClientError::UrlEncodingFailed,
            Self::RequestNotSent(reason) => ApiClientError::RequestNotSent(reason.clone()),
            Self::ResponseDecodingFailed => ApiClientError::ResponseDecodingFailed,
            Self::RequestTimeoutReceived => ApiClientError::RequestTimeoutReceived,
            Self::ConnectionClosedIncompleteMessage => {
                ApiClientError::ConnectionClosedIncompleteMessage
            }
            Self::InternalServerErrorReceived => ApiClientError::InternalServerErrorReceived,
            Self::BadGatewayReceived => ApiClientError::BadGatewayReceived,
            Self::ServiceUnavailableReceived => ApiClientError::ServiceUnavailableReceived,
            Self::GatewayTimeoutReceived => ApiClientError::GatewayTimeoutReceived,
            Self::UnexpectedServerResponse => ApiClientError::UnexpectedServerResponse,
        }
    }
}
