use actix_web::HttpResponse;

use crate::openapi::create_openapi;

pub async fn get_openapi_spec() -> HttpResponse {
    let openapi = create_openapi();
    HttpResponse::Ok().json(&openapi)
}
