use actix_web::http::StatusCode;

pub mod transformers;

#[derive(Debug)]
pub enum ApplicationResponse<R> {
    Json {
        data: R,
        status: StatusCode,
    },
    StatusOk,
    TextPlain(String),
    FileDownload {
        data: Vec<u8>,
        filename: String,
        content_type: mime::Mime,
    },
}

impl<R> ApplicationResponse<R> {
    /// Create a JSON response with 200 OK status
    pub fn json(data: R) -> Self {
        Self::Json {
            data,
            status: StatusCode::OK,
        }
    }

    /// Create a JSON response with 201 Created status
    pub fn json_created(data: R) -> Self {
        Self::Json {
            data,
            status: StatusCode::CREATED,
        }
    }
}

// Type alias for our responses
pub type ReconResponse<T> =
    error_stack::Result<ApplicationResponse<T>, crate::errors::ReconAppError>;
