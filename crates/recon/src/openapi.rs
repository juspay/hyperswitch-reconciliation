use utoipa::{
    openapi::{OpenApi, OpenApiBuilder},
    OpenApi as OpenApiTrait,
};

use crate::routes::{
    account::AccountApi, audit_trail::AuditTrailApi, auth::AuthApi,
    business_profile::BusinessProfileApi, entry::EntryApi,
    exception_management::ExceptionManagementApi, health::HealthApi, ingestion::IngestionApi,
    link_config::LinkConfigApi, linking_processor::LinkingProcessorApi,
    merchant_account::MerchantAccountApi, metadata_schema::MetadataSchemaApi,
    overview::OverviewApi, recon_engine::ReconEngineApi, recon_rule::ReconRuleApi,
    reporting::ReportApi, search_entry_key::SearchEntryKeyApi,
    search_expectation::SearchExpectationApi, search_group::SearchGroupApi,
    staging_entry::StagingEntryApi, transaction::TransactionApi, transformation::TransformationApi,
};

/// Creates the complete OpenAPI documentation by merging all module APIs
///
/// # Modular OpenAPI Pattern
///
/// This function implements a modular approach to OpenAPI documentation where:
/// 1. Each route module defines its own OpenAPI struct with `#[derive(OpenApi)]`
/// 2. Module APIs are merged here to create the complete documentation
/// 3. Base information (title, version, etc.) is defined centrally
///
/// ## Adding New Modules
///
/// To add a new module's API documentation:
/// 1. In your route module, create an OpenAPI struct:
///    ```rust,ignore
///    #[derive(OpenApi)]
///    #[openapi(
///        paths(your_endpoint_function),
///        components(schemas(YourRequestType, YourResponseType)),
///        tags((name = "Your Module", description = "Description"))
///    )]
///    pub struct YourModuleApi;
///    ```
/// 2. Import it in this file and add `openapi.merge(YourModuleApi::openapi());`
///
/// ## Benefits
/// - Each module manages its own API documentation
/// - Documentation lives close to implementation
/// - Scalable as the project grows
/// - Avoids merge conflicts on a single large file
/// - Easy to find and maintain module-specific docs
pub fn create_openapi() -> OpenApi {
    // Create base OpenAPI with common information
    let mut openapi = OpenApiBuilder::new()
        .info(
            utoipa::openapi::InfoBuilder::new()
                .title("Hyperswitch Recon API")
                .version("0.1.0")
                .description(Some("API for merchant and business profile management with payment reconciliation capabilities"))
                .contact(Some(
                    utoipa::openapi::ContactBuilder::new()
                        .name(Some("Hyperswitch Team"))
                        .build()
                ))
                .license(Some(
                    utoipa::openapi::LicenseBuilder::new()
                        .name("Apache 2.0")
                        .build()
                ))
                .build()
        )
        .components(Some(
            utoipa::openapi::ComponentsBuilder::new()
                .security_scheme(
                    "Bearer",
                    utoipa::openapi::security::SecurityScheme::Http(
                        utoipa::openapi::security::HttpBuilder::new()
                            .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                            .bearer_format("JWT")
                            .description(Some("JWT token for authentication. Obtain token from /auth/generate_token endpoint."))
                            .build()
                    )
                )
                .security_scheme(
                    "ApiKey",
                    utoipa::openapi::security::SecurityScheme::ApiKey(
                        utoipa::openapi::security::ApiKey::Header(
                            utoipa::openapi::security::ApiKeyValue::new("api-key")
                        )
                    )
                )
                .security_scheme(
                    "InternalApiKey",
                    utoipa::openapi::security::SecurityScheme::ApiKey(
                        utoipa::openapi::security::ApiKey::Header(
                            utoipa::openapi::security::ApiKeyValue::with_description(
                                "x-internal-api-key",
                                "Service-to-service key. Held only by the recon service itself."
                            )
                        )
                    )
                )
                .build()
        ))
        .build();

    // Merge module APIs
    openapi.merge(HealthApi::openapi());
    openapi.merge(MerchantAccountApi::openapi());
    openapi.merge(BusinessProfileApi::openapi());
    openapi.merge(AccountApi::openapi());
    openapi.merge(AuthApi::openapi());
    openapi.merge(StagingEntryApi::openapi());
    openapi.merge(TransactionApi::openapi());
    openapi.merge(EntryApi::openapi());
    openapi.merge(OverviewApi::openapi());
    openapi.merge(ReconRuleApi::openapi());
    openapi.merge(LinkConfigApi::openapi());
    openapi.merge(ReconEngineApi::openapi());
    openapi.merge(LinkingProcessorApi::openapi());
    openapi.merge(IngestionApi::openapi());
    openapi.merge(TransformationApi::openapi());
    openapi.merge(ReportApi::openapi());
    openapi.merge(MetadataSchemaApi::openapi());
    openapi.merge(ExceptionManagementApi::openapi());
    openapi.merge(AuditTrailApi::openapi());
    openapi.merge(SearchGroupApi::openapi());
    openapi.merge(SearchEntryKeyApi::openapi());
    openapi.merge(SearchExpectationApi::openapi());

    openapi
}
