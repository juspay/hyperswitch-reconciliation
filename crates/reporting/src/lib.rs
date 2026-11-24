//! Reporting service: generates reports and delivers them, driven by Invokr callbacks.
//!
//! This crate must not depend on `recon`. It shares the recon database through `storage_impl`.

use actix_web::{
    body::MessageBody,
    dev::{self, ServiceFactory, ServiceRequest},
    http::StatusCode,
    middleware::ErrorHandlers,
    web, App,
};
use recon_interfaces::secrets_interface::secret_state::{RawSecret, SecuredSecret};
use storage_impl::Store;
use tracing_actix_web::TracingLogger;

pub mod config;
pub mod core;
pub mod errors;
pub mod openapi;
pub mod routes;
pub mod secrets_transformers;
pub mod services;
pub mod types;

use routes::AppState;

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
    App::new()
        .app_data(web::Data::new(state.clone()))
        .wrap(recon_env::request_id::RequestId)
        .wrap(TracingLogger::default())
        .wrap(ErrorHandlers::new().handler(
            StatusCode::METHOD_NOT_ALLOWED,
            errors::custom_error_handlers,
        ))
        .service(routes::app::Health::server(state.clone()))
        .service(routes::app::ApiDocs::server(state))
        .default_service(web::route().to(errors::default_handler))
}

/// Builds the server from a loaded configuration.
/// Returns the server, the port it listens on, and the decrypted config.
pub async fn initialize_server(
    secured_config: config::Config<SecuredSecret>,
) -> Result<(dev::Server, u16, config::Config<RawSecret>), Box<dyn std::error::Error>> {
    let secret_management_client = secured_config
        .secrets_management
        .get_secret_management_client()
        .await
        .map_err(|e| format!("Failed to create secret management client: {e}"))?;

    let config =
        secrets_transformers::fetch_raw_secrets(secured_config, &*secret_management_client)
            .await
            .map_err(|e| format!("Failed to decrypt secrets: {e}"))?;

    let store = Store::new(&config.database.get_inner().clone().into())
        .await
        .map_err(|e| format!("Failed to create database store: {e:?}"))?;

    let state = AppState {
        config: config.clone(),
        store,
    };

    let listener = std::net::TcpListener::bind((config.server.host.as_str(), config.server.port))
        .map_err(|e| {
        format!(
            "Failed to bind to {}:{} - {}",
            config.server.host, config.server.port, e
        )
    })?;

    // The actual port, which differs from the configured one when that is 0
    let port = listener.local_addr()?.port();

    let server = actix_web::HttpServer::new(move || make_app(state.clone()))
        .listen(listener)?
        .workers(config.server.workers)
        .shutdown_timeout(config.server.shutdown_timeout)
        .run();

    Ok((server, port, config))
}
