use actix_web::http::StatusCode;

pub mod health;

#[derive(Debug)]
pub enum ApplicationResponse<R> {
    Json { data: R, status: StatusCode },
    TextPlain(String),
}

impl<R> ApplicationResponse<R> {
    /// A JSON response with 200 OK.
    pub fn json(data: R) -> Self {
        Self::Json {
            data,
            status: StatusCode::OK,
        }
    }
}

pub type ReportingResponse<T> =
    Result<ApplicationResponse<T>, error_stack::Report<crate::errors::ReportingAppError>>;
