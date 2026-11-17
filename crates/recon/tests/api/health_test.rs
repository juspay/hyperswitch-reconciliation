use api_models::health::DeepHealthResponse;
use common_enums::HealthComponent;
use strum::IntoEnumIterator;

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
    let response = client.get("/health/deep").send().await;

    // Assert
    assert_eq!(response.status(), 200);

    let body: DeepHealthResponse = response.json().await.unwrap();
    assert_eq!(body.components.len(), HealthComponent::iter().count());
    assert!(body.is_healthy());
}
