use actix_web::{web, HttpRequest, HttpResponse};
use api_models::errors::ApiErrorResponse as ApiErrorPayload;
use common_utils::{errors::ErrorSwitch, logging};
use error_stack::Report;
use http::header;
use std::future::Future;

use crate::{routes::AppState, services::auth::AuthValidator, types::ApplicationResponse};

pub async fn server_wrap<F, Fut, T, E, A>(
    auth: A,
    req: &HttpRequest,
    state: web::Data<AppState>,
    func: F,
) -> HttpResponse
where
    A: AuthValidator,
    F: FnOnce(AppState, A::AuthData) -> Fut,
    Fut: Future<Output = Result<ApplicationResponse<T>, Report<E>>>,
    T: serde::Serialize,
    E: ErrorSwitch<ApiErrorPayload> + std::error::Error + Send + Sync + 'static,
{
    // Perform authentication
    let auth_data = match auth.validate(req, state.get_ref().clone()).await {
        Ok(data) => data,
        Err(auth_error) => {
            return log_and_return_error_response(auth_error);
        }
    };

    // Call the handler with validated auth data
    match func(state.as_ref().clone(), auth_data).await {
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
        ApplicationResponse::StatusOk => HttpResponse::Ok().finish(),
        ApplicationResponse::TextPlain(text) => {
            HttpResponse::Ok().content_type("text/plain").body(text)
        }
        ApplicationResponse::FileDownload {
            data,
            filename,
            content_type,
        } => HttpResponse::Ok()
            .content_type(content_type)
            .insert_header((
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ))
            .body(data),
    }
}
