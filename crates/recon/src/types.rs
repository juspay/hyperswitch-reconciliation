use actix_web::http::StatusCode;

pub mod transformers;

#[derive(Debug)]
pub enum ApplicationResponse<R> {
    Json {
        data: R,
        status: StatusCode,
    },
    JsonNoStore {
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

    pub fn json_created_no_store(data: R) -> Self {
        Self::JsonNoStore {
            data,
            status: StatusCode::CREATED,
        }
    }
}

// Type alias for our responses
pub type ReconResponse<T> =
    Result<ApplicationResponse<T>, error_stack::Report<crate::errors::ReconAppError>>;
