use std::future::Future;

use actix_web::{web, HttpResponse};
use api_models::errors::ApiErrorResponse as ApiErrorPayload;
use common_utils::errors::ErrorSwitch;
use error_stack::Report;
use recon_env::logging;

use crate::{routes::AppState, types::ApplicationResponse};

/// Runs a handler and turns its result into an HTTP response.
/// Authentication is added here once the Invokr callback endpoints exist.
pub async fn server_wrap<F, Fut, T, E>(state: web::Data<AppState>, func: F) -> HttpResponse
where
    F: FnOnce(AppState) -> Fut,
    Fut: Future<Output = Result<ApplicationResponse<T>, Report<E>>>,
    T: serde::Serialize,
    E: ErrorSwitch<ApiErrorPayload> + std::error::Error + Send + Sync + 'static,
{
    match func(state.as_ref().clone()).await {
        Ok(response) => http_response_from_application(response),
        Err(error) => log_and_return_error_response(error),
    }
}

pub fn log_and_return_error_response<E>(error: Report<E>) -> HttpResponse
where
    E: ErrorSwitch<ApiErrorPayload> + std::error::Error + Send + Sync + 'static,
{
    logging::error!(?error);
    ErrorSwitch::<ApiErrorPayload>::switch(error.current_context()).error_response()
}

pub fn http_response_from_application<T: serde::Serialize>(
    response: ApplicationResponse<T>,
) -> HttpResponse {
    match response {
        ApplicationResponse::Json { data, status } => HttpResponse::build(status).json(data),
        ApplicationResponse::TextPlain(text) => {
            HttpResponse::Ok().content_type("text/plain").body(text)
        }
    }
}
