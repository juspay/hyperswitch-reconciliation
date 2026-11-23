use utoipa::{
    openapi::{InfoBuilder, OpenApi, OpenApiBuilder},
    OpenApi as OpenApiTrait,
};

use crate::routes::health::HealthApi;

/// Builds the OpenAPI document. Each route module defines its own `#[derive(OpenApi)]` struct,
/// and this function merges them.
pub fn create_openapi() -> OpenApi {
    let mut openapi = OpenApiBuilder::new()
        .info(
            InfoBuilder::new()
                .title("Hyperswitch Reporting Service API")
                .version(env!("CARGO_PKG_VERSION"))
                .description(Some(
                    "Endpoints Invokr calls to generate and deliver reports",
                ))
                .build(),
        )
        .build();

    openapi.merge(HealthApi::openapi());
    openapi
}
