use api_models::health::{DeepHealthResponse, HealthStatus};

use crate::common::TestApp;
use pretty_assertions::assert_eq;

#[actix_web::test]
async fn health_check_works() {
    // Arrange
    let app = TestApp::new().await;
    let client = app.api_client();

    // Act
    let response = client.get("/health").send().await;

    // Assert
    assert_eq!(response.status(), 200);

    let body = response.text().await.unwrap();
    assert_eq!(body, "OK");
}

#[actix_web::test]
async fn health_check_with_wrong_method_returns_405() {
    // Arrange
    let app = TestApp::new().await;
    let client = app.api_client();

    // Act - Send POST instead of GET
    let response = client.post("/health").send().await;

    // Assert
    assert_eq!(response.status(), 405);
}

#[actix_web::test]
async fn deep_health_reports_each_component() {
    // Arrange
    let app = TestApp::new().await;
    let client = app.api_client();

    // Act
    let response = client.get("/deep_health").send().await;

    // Assert
    assert_eq!(response.status(), 200);

    let body: DeepHealthResponse = response.json().await.unwrap();
    assert_eq!(body.status, HealthStatus::Healthy);
    assert_eq!(body.database.status, HealthStatus::Healthy);
    assert_eq!(body.encryption.status, HealthStatus::NotApplicable);
    assert_eq!(body.user_client.status, HealthStatus::NotApplicable);
    assert_eq!(body.recon_processor.status, HealthStatus::Healthy);
    assert_eq!(body.linking_processor.status, HealthStatus::Healthy);
}
