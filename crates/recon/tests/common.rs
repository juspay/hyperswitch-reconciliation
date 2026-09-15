#![allow(
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common_utils::{id_generation::generate_id_with_default_len, id_type::MerchantId};
use external_services::{user::client::mock_user_client::MockUserClient, UserResponse};
use jsonwebtoken::{encode, EncodingKey, Header};
use masking::PeekInterface;
use recon::{config::Config, initialize_server, services::auth::Claims};
use recon_interfaces::secrets_interface::secret_state::SecuredSecret;

pub struct TestApp {
    pub base_url: String,
    pub config: Config<SecuredSecret>,
}

impl TestApp {
    pub async fn new() -> Self {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root = std::path::Path::new(manifest_dir)
            .parent()
            .expect("Failed to get parent")
            .parent()
            .expect("Failed to get workspace root");

        let config_path = workspace_root.join("config").join("test.toml");

        // Load config with environment variable support
        let secured_config =
            Config::with_config_path(Some(config_path)).expect("Failed to load test configuration");

        let (server, port, config) = initialize_server(secured_config.clone())
            .await
            .expect("Failed to initialize test server");

        tokio::spawn(server);

        tokio::time::sleep(Duration::from_millis(100)).await;

        let base_url = format!("http://{}:{}", config.server.host, port);

        Self {
            base_url,
            config: secured_config,
        }
    }

    pub fn api_client(&self) -> TestClient {
        TestClient {
            base_url: self.base_url.clone(),
            client: reqwest::Client::new(),
            config: self.config.clone(),
        }
    }
}

pub struct TestClient {
    base_url: String,
    client: reqwest::Client,
    config: Config<SecuredSecret>,
}

impl TestClient {
    /// Parse response as success type T or panic with error details
    pub async fn expect_success<T: for<'de> serde::Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> T {
        let status = response.status();
        assert!(status.is_success(), "Expected success, got {status}");
        response
            .json::<T>()
            .await
            .unwrap_or_else(|e| panic!("Failed to parse success response: {e}"))
    }

    /// Parse response expecting an error, return error message string
    pub async fn expect_error(&self, response: reqwest::Response, expected_status: u16) -> String {
        assert_eq!(response.status(), expected_status);

        // The actual response structure is {"error": {...}}
        let error_wrapper: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|e| panic!("Failed to parse error response: {e}"));

        // Extract the error message from the nested structure
        let error_obj = error_wrapper
            .get("error")
            .unwrap_or_else(|| panic!("Error response missing 'error' field: {error_wrapper}"));

        error_obj
            .get("message")
            .and_then(|msg| msg.as_str())
            .unwrap_or_else(|| panic!("Error response missing message field: {error_obj}"))
            .to_string()
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        RequestBuilder {
            client: self.client.clone(),
            method: reqwest::Method::GET,
            url: format!("{}{}", self.base_url, path),
            headers: reqwest::header::HeaderMap::new(),
            request_builder: None,
        }
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        RequestBuilder {
            client: self.client.clone(),
            method: reqwest::Method::POST,
            url: format!("{}{}", self.base_url, path),
            headers: reqwest::header::HeaderMap::new(),
            request_builder: None,
        }
    }

    pub fn put(&self, path: &str) -> RequestBuilder {
        RequestBuilder {
            client: self.client.clone(),
            method: reqwest::Method::PUT,
            url: format!("{}{}", self.base_url, path),
            headers: reqwest::header::HeaderMap::new(),
            request_builder: None,
        }
    }

    /// Setup a complete test merchant with profile and return JWT token
    pub async fn bootstrap(
        &self,
        merchant_name: &str,
        profile_name: &str,
    ) -> (MerchantId, String, String) {
        use api_models::auth::{TestMerchantSetupRequest, TestMerchantSetupResponse};

        let request = TestMerchantSetupRequest {
            merchant_name: merchant_name.to_string(),
            profile_name: profile_name.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/auth/bootstrap", self.base_url))
            .header("api-key", "test_admin_12345678901234567890")
            .json(&request)
            .send()
            .await
            .expect("Failed to setup test merchant");

        assert_eq!(response.status(), 200, "Setup test merchant should succeed");

        let setup_response: TestMerchantSetupResponse = response
            .json()
            .await
            .expect("Failed to parse setup response");

        let exp = SystemTime::now()
            .checked_add(Duration::from_secs(
                self.config.auth.get_inner().jwt_expiry_hours * 3600,
            ))
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            merchant_id: setup_response.merchant_id.clone(),
            profile_id: setup_response.profile_id.clone(),
            user_id: generate_id_with_default_len("user"),
            exp,
        };

        let encoding_key =
            EncodingKey::from_secret(self.config.auth.get_inner().jwt_secret.peek().as_ref());
        let token = encode(&Header::default(), &claims, &encoding_key).unwrap();

        (setup_response.merchant_id, setup_response.profile_id, token)
    }

    pub fn get_test_user_details(user_id: &str) -> UserResponse {
        let mock_user_client = MockUserClient;
        mock_user_client.create_active_mock_user(user_id)
    }
}

pub struct RequestBuilder {
    client: reqwest::Client,
    method: reqwest::Method,
    url: String,
    headers: reqwest::header::HeaderMap,
    request_builder: Option<reqwest::RequestBuilder>,
}

impl RequestBuilder {
    pub fn json<T: serde::Serialize>(mut self, body: &T) -> Self {
        let mut request = self.client.request(self.method.clone(), &self.url);
        request = request.json(body);

        for (key, value) in self.headers.iter() {
            request = request.header(key, value);
        }

        self.request_builder = Some(request);
        self
    }

    pub async fn send(self) -> reqwest::Response {
        let request = if let Some(builder) = self.request_builder {
            builder
        } else {
            let mut request = self.client.request(self.method, &self.url);
            for (key, value) in self.headers.iter() {
                request = request.header(key, value);
            }
            request
        };

        request.send().await.expect("Failed to send request")
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(
            reqwest::header::HeaderName::from_bytes(key.as_bytes()).expect("Invalid header name"),
            reqwest::header::HeaderValue::from_str(value).expect("Invalid header value"),
        );
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        let mut request = self.client.request(self.method.clone(), &self.url);
        request = request.body(body.to_string());

        for (key, value) in self.headers.iter() {
            request = request.header(key, value);
        }

        self.request_builder = Some(request);
        self
    }

    pub fn with_auth(self, token: &str) -> Self {
        self.header("Authorization", &format!("Bearer {token}"))
    }

    pub fn with_admin_key(self) -> Self {
        // Use the admin API key from test config
        self.header("api-key", "test_admin_12345678901234567890")
    }
}
