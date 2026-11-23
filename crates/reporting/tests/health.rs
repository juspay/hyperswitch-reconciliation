#![allow(
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::time::Duration;

use pretty_assertions::assert_eq;
use recon_interfaces::secrets_interface::secret_state::SecuredSecret;
use reporting::{
    config::Config,
    initialize_server,
    types::health::{ComponentHealth, DeepHealthResponse, HealthComponent},
};
use strum::IntoEnumIterator;

fn test_config() -> Config<SecuredSecret> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("Failed to get workspace root");
    let config_path = workspace_root
        .join("config")
        .join("reporting")
        .join("test.toml");
    Config::with_config_path(Some(config_path)).expect("Failed to load test configuration")
}

/// Starts the server and returns its base URL.
async fn spawn_app(config: Config<SecuredSecret>) -> String {
    let (server, port, config) = initialize_server(config)
        .await
        .expect("Failed to initialize test server");
    tokio::spawn(server);
    tokio::time::sleep(Duration::from_millis(100)).await;
    format!("http://{}:{}", config.server.host, port)
}

#[actix_web::test]
async fn health_check_works() {
    let base_url = spawn_app(test_config()).await;

    let response = reqwest::get(format!("{base_url}/health")).await.unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "OK");
}

#[actix_web::test]
async fn deep_health_reports_database_healthy() {
    let base_url = spawn_app(test_config()).await;

    let response = reqwest::get(format!("{base_url}/health/deep"))
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: DeepHealthResponse = response.json().await.unwrap();
    assert_eq!(
        body.components,
        HealthComponent::iter()
            .map(|component| ComponentHealth::Healthy { component })
            .collect::<Vec<_>>()
    );
}

#[actix_web::test]
async fn openapi_spec_lists_health_paths() {
    let base_url = spawn_app(test_config()).await;

    let response = reqwest::get(format!("{base_url}/api_docs/openapi.json"))
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let spec: serde_json::Value = response.json().await.unwrap();
    let paths = spec
        .get("paths")
        .and_then(serde_json::Value::as_object)
        .expect("spec has no paths");
    assert!(paths.contains_key("/health"));
    assert!(paths.contains_key("/health/deep"));
}
