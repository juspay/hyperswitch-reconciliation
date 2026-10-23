use actix_web::{
    body::MessageBody,
    dev::{self, ServiceFactory, ServiceRequest},
    middleware::ErrorHandlers,
    web, App,
};
use async_bb8_diesel::ConnectionManager;
use bb8::Pool;
use common_enums::{LinkingProcessorStatus, ReconProcessorStatus};
use diesel::PgConnection;
use external_services::{faas::FaasHandler, file_storage::FileStorageHandler};
use http::StatusCode;
use recon_interfaces::secrets_interface::secret_state::{RawSecret, SecuredSecret};
use routes::AppState;
use tokio::sync::watch;
use tracing_actix_web::TracingLogger;

pub mod config;
pub mod core;
pub mod db;
pub mod domain;
pub mod env;
pub mod errors;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod secrets_transformers;
pub mod services;
pub mod types;
pub mod utils;

pub fn make_app(
    state: AppState,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let mut app = App::new()
        .app_data(web::Data::new(state.clone()))
        .wrap(middleware::RequestId)
        .wrap(TracingLogger::default())
        .wrap(ErrorHandlers::new().handler(
            StatusCode::METHOD_NOT_ALLOWED,
            errors::custom_error_handlers,
        ));

    app = app.service(routes::app::Health::server(state.clone()));
    app = app.service(routes::app::MerchantAccount::server(state.clone()));
    app = app.service(routes::app::BusinessProfile::server(state.clone()));
    app = app.service(routes::app::Account::server(state.clone()));
    app = app.service(routes::app::AuditTrail::server(state.clone()));
    app = app.service(routes::app::Auth::server(state.clone()));
    app = app.service(routes::app::StagingEntry::server(state.clone()));
    app = app.service(routes::app::Transaction::server(state.clone()));
    app = app.service(routes::app::Entry::server(state.clone()));
    app = app.service(routes::app::Overview::server(state.clone()));
    app = app.service(routes::app::ReconRule::server(state.clone()));
    app = app.service(routes::app::LinkConfig::server(state.clone()));
    app = app.service(routes::app::ReconEngine::server(state.clone()));
    app = app.service(routes::app::LinkingProcessor::server(state.clone()));
    app = app.service(routes::app::Ingestion::server(state.clone()));
    app = app.service(routes::app::Transformation::server(state.clone()));
    app = app.service(routes::app::Report::server(state.clone()));
    app = app.service(routes::app::MetadataSchema::server(state.clone()));
    app = app.service(routes::app::SearchGroup::server(state.clone()));
    app = app.service(routes::app::SearchEntryKey::server(state.clone()));
    app = app.service(routes::app::SearchExpectation::server(state.clone()));
    app = app.service(routes::app::ExceptionManagement::server(state.clone()));
    app = app.service(routes::app::ApiDocs::server(state.clone()));

    // Add default service for unmatched routes
    app = app.default_service(web::route().to(errors::default_handler));

    app
}

/// Initialize the server with a pre-loaded configuration that respects environment variables
/// Returns the server instance, the actual port it's listening on, and the loaded config
pub async fn initialize_server(
    secured_config: config::Config<SecuredSecret>,
) -> Result<(dev::Server, u16, config::Config<RawSecret>), Box<dyn std::error::Error>> {
    // Initialize secret management client
    let secret_management_client = secured_config
        .secrets_management
        .get_secret_management_client()
        .await
        .map_err(|e| format!("Failed to create secret management client: {e}"))?;

    // Transform secrets to raw state
    let config =
        secrets_transformers::fetch_raw_secrets(secured_config, &*secret_management_client)
            .await
            .map_err(|e| format!("Failed to decrypt secrets: {e}"))?;

    // Create connection pool
    let manager =
        ConnectionManager::<PgConnection>::new(config.database.get_inner().get_database_url());
    let pool = Pool::builder()
        .build(manager)
        .await
        .map_err(|e| format!("Failed to create connection pool: {e}"))?;

    // Create file storage handler
    let file_storage = FileStorageHandler::from_config(&config.file_storage)
        .await
        .map_err(|e| format!("Failed to create file storage client: {e}"))?;

    // Create FaaS client for report generation
    let faas = FaasHandler::from_config(&config.faas)
        .await
        .map_err(|e| format!("Failed to create FaaS client: {e}"))?;

    // Create SFTP file storage handler for ingestion
    let sftp_file_storage =
        FileStorageHandler::from_config(&config.ingestion_service.sftp_file_storage)
            .await
            .map_err(|e| format!("Failed to create SFTP file storage client for ingestion: {e}"))?;

    let ingestion = routes::IngestionState {
        config: config.ingestion_service.clone(),
        sftp_file_storage,
    };

    let (sx, rx) = watch::channel(ReconProcessorStatus::Stopped);
    let (linking_sx, linking_rx) = watch::channel(LinkingProcessorStatus::Stopped);

    // Create encryption client
    let encryption_client = config
        .application_encryption.get_inner()
        .get_encryption_client()
        .await
        .map_err(|e| {
            format!(
                "Failed to initialize application encryption client. Please verify your encryption configuration. Error: {e}"
            )
        })?;

    // Create HyperSwitch client
    let user_client = config.user_client.get_inner().get_user_client();

    // Create app state
    let state = AppState {
        store: db::Store { master_pool: pool },
        config: config.clone(),
        file_storage,
        faas,
        ingestion,
        recon_processor_status: sx,
        linking_processor_status: linking_sx,
        encryption_client,
        user_client,
    };

    tokio::spawn(core::recon_engine::recon_processor_v2(state.clone(), rx));
    tokio::spawn(core::linking_processor::linking_processor(
        state.clone(),
        linking_rx,
    ));

    // Bind to the configured address and port
    let listener = std::net::TcpListener::bind((config.server.host.as_str(), config.server.port))
        .map_err(|e| {
        format!(
            "Failed to bind to {}:{} - {}",
            config.server.host, config.server.port, e
        )
    })?;

    // Get the actual port (important when config.server.port is 0)
    let port = listener.local_addr()?.port();

    // Create and configure the server
    let server = actix_web::HttpServer::new(move || make_app(state.clone()))
        .listen(listener)?
        .workers(config.server.workers)
        .shutdown_timeout(config.server.shutdown_timeout)
        .run();

    Ok((server, port, config))
}
