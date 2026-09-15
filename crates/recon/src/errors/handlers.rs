use super::ReconAppError;
use actix_web::{
    dev::ServiceResponse, middleware::ErrorHandlerResponse, HttpRequest, HttpResponse,
    ResponseError, Result as ActixResult,
};
use http::StatusCode;

pub fn custom_error_handlers<B: actix_web::body::MessageBody + 'static>(
    res: ServiceResponse<B>,
) -> ActixResult<ErrorHandlerResponse<B>> {
    let original_status = res.status();
    let (req, service_resp_part) = res.into_parts();

    let app_error_to_respond_with = match original_status {
        StatusCode::NOT_FOUND => ReconAppError::InvalidRequestUrl,
        StatusCode::METHOD_NOT_ALLOWED => ReconAppError::InvalidHttpMethod,
        _ => ReconAppError::InvalidRequestUrl,
    };

    tracing::warn!(
        target: "recon_api_error_handler",
        error = %app_error_to_respond_with,
        "Client error occurred"
    );

    let final_http_response = match service_resp_part.error() {
        Some(_actix_error_already_in_res) => service_resp_part.map_into_boxed_body(),
        None => app_error_to_respond_with
            .error_response()
            .map_into_boxed_body(),
    };

    let new_service_response = ServiceResponse::new(req, final_http_response).map_into_right_body();

    Ok(ErrorHandlerResponse::Response(new_service_response))
}

/// Default handler for unmatched routes.
pub async fn default_handler(_req: HttpRequest) -> HttpResponse {
    tracing::warn!(
        target: "recon_api_error_handler",
        "Route does not exist"
    );
    ReconAppError::InvalidRequestUrl.error_response()
}
