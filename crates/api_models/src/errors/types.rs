use std::borrow::Cow;

use actix_web::{http::header, HttpResponse};
use http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiError {
    pub sub_code: &'static str,
    pub error_identifier: u16,
    pub error_message: String,
    pub extra: Option<Extra>,
}

impl ApiError {
    pub fn new(
        sub_code: &'static str,
        error_identifier: u16,
        error_message: impl ToString,
        extra: Option<Extra>,
    ) -> Self {
        Self {
            sub_code,
            error_identifier,
            error_message: error_message.to_string(),
            extra,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Extra {
    pub stacktrace: Option<String>,
}

#[derive(Debug, Serialize)]
struct SerializableErrorWrapper<'a> {
    #[serde(rename = "type")]
    error_type_category: &'static str,
    message: Cow<'a, str>,
    code: String,
    #[serde(flatten)]
    extra: &'a Option<Extra>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum ApiErrorResponse {
    Unauthorized(ApiError),
    ForbiddenCommonResource(ApiError),
    Conflict(ApiError),
    Gone(ApiError),
    NotFound(ApiError),
    MethodNotAllowed(ApiError),
    BadRequest(ApiError),
    InternalServerError(ApiError),
    BadGateway(ApiError),
}

impl ::core::fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_info = self.get_internal_error();
        let wrapper = SerializableErrorWrapper {
            error_type_category: self.error_type_category(),
            message: Cow::Borrowed(error_info.error_message.as_str()),
            code: format!("{}_{:02}", error_info.sub_code, error_info.error_identifier),
            extra: &error_info.extra,
        };
        write!(
            f,
            r#"{{"error":{}}}"#,
            serde_json::to_string(&wrapper)
                .unwrap_or_else(|_| "API error response serialization failed".to_string())
        )
    }
}

impl ApiErrorResponse {
    // Helper to get the internal ApiError struct from any variant
    pub(crate) fn get_internal_error(&self) -> &ApiError {
        match self {
            Self::Unauthorized(i)
            | Self::ForbiddenCommonResource(i)
            | Self::Conflict(i)
            | Self::Gone(i)
            | Self::NotFound(i)
            | Self::MethodNotAllowed(i)
            | Self::BadRequest(i)
            | Self::InternalServerError(i)
            | Self::BadGateway(i) => i,
        }
    }

    // Determines the category string for the "type" field in JSON
    pub(crate) fn error_type_category(&self) -> &'static str {
        match self {
            Self::Unauthorized(_)
            | Self::ForbiddenCommonResource(_)
            | Self::Conflict(_)
            | Self::Gone(_)
            | Self::MethodNotAllowed(_)
            | Self::NotFound(_)
            | Self::BadRequest(_) => "invalid_request",
            Self::InternalServerError(_) | Self::BadGateway(_) => "api_error",
        }
    }

    // Returns the appropriate HTTP status code for each error variant
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::ForbiddenCommonResource(_) => StatusCode::FORBIDDEN,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Gone(_) => StatusCode::GONE,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
        }
    }

    pub fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(header::ContentType::json())
            .body(self.to_string())
    }
}

impl std::error::Error for ApiErrorResponse {}
