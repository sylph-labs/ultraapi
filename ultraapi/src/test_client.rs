//! TestClient - Built-in test client for UltraAPI applications
//! 
//! This module provides a TestClient similar to FastAPI's TestClient,
//! allowing you to test your UltraAPI applications without running an actual server.

use crate::UltraApiApp;
use axum::Router;
use reqwest::Client;
use tokio::net::TcpListener;
use tokio::spawn;

/// Test client for UltraAPI applications
/// 
/// Provides a convenient way to test your API without running an actual server.
/// Automatically spawns a test server on a random port.
/// 
/// # Example
/// 
/// ```ignore
/// use ultraapi::prelude::*;
/// 
/// #[get("/hello")]
/// async fn hello() -> String {
///     "Hello, World!".to_string()
/// }
/// 
/// #[tokio::test]
/// async fn test_hello() {
///     let app = UltraApiApp::new().into_router();
///     let client = TestClient::new(app);
///     
///     let response = client.get("/hello").await;
///     assert_eq!(response.status(), 200);
///     
///     let body = response.text().await.unwrap();
///     assert_eq!(body, "Hello, World!");
/// }
/// ```
pub struct TestClient {
    /// The underlying reqwest client
    client: Client,
    /// Base URL of the test server
    base_url: String,
    /// Handle to the spawned server task
    _server_handle: tokio::task::JoinHandle<()>,
}

impl TestClient {
    /// Create a new TestClient from an UltraApiApp (async)
    /// 
    /// This converts the app to a Router and starts a test server.
    /// 
    /// # Arguments
    /// 
    /// * `app` - An UltraApiApp instance
    /// 
    /// # Returns
    /// 
    /// A TestClient instance connected to a running test server
    pub async fn new(app: UltraApiApp) -> Self {
        let router: Router = app.into_router();
        Self::new_router(router).await
    }

    /// Create a new TestClient from a Router (async)
    /// 
    /// # Arguments
    /// 
    /// * `router` - An axum Router instance
    /// 
    /// # Returns
    /// 
    /// A TestClient instance connected to a running test server
    pub async fn new_router(router: Router) -> Self {
        // Create a TCP listener on a random port
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind to random port");
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        // Spawn the server
        let server_handle = spawn(async move {
            axum::serve(listener, router).await.expect("Server error");
        });

        // Create the reqwest client
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            base_url,
            _server_handle: server_handle,
        }
    }

    /// Get the base URL of the test server
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Send a GET request
    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client
            .get(format!("{}{}", self.base_url, path))
            .send()
            .await
            .expect("GET request failed")
    }

    /// Send a POST request
    pub async fn post<B: serde::Serialize>(&self, path: &str, body: &B) -> reqwest::Response {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await
            .expect("POST request failed")
    }

    /// Send a POST request with raw body
    pub async fn post_raw(&self, path: &str, body: impl Into<reqwest::Body>) -> reqwest::Response {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .body(body)
            .send()
            .await
            .expect("POST request failed")
    }

    /// Send a PUT request
    pub async fn put<B: serde::Serialize>(&self, path: &str, body: &B) -> reqwest::Response {
        self.client
            .put(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await
            .expect("PUT request failed")
    }

    /// Send a PUT request with raw body
    pub async fn put_raw(&self, path: &str, body: impl Into<reqwest::Body>) -> reqwest::Response {
        self.client
            .put(format!("{}{}", self.base_url, path))
            .body(body)
            .send()
            .await
            .expect("PUT request failed")
    }

    /// Send a DELETE request
    pub async fn delete(&self, path: &str) -> reqwest::Response {
        self.client
            .delete(format!("{}{}", self.base_url, path))
            .send()
            .await
            .expect("DELETE request failed")
    }

    /// Send a PATCH request
    pub async fn patch<B: serde::Serialize>(&self, path: &str, body: &B) -> reqwest::Response {
        self.client
            .patch(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await
            .expect("PATCH request failed")
    }

    /// Send a PATCH request with raw body
    pub async fn patch_raw(&self, path: &str, body: impl Into<reqwest::Body>) -> reqwest::Response {
        self.client
            .patch(format!("{}{}", self.base_url, path))
            .body(body)
            .send()
            .await
            .expect("PATCH request failed")
    }

    /// Send a HEAD request
    pub async fn head(&self, path: &str) -> reqwest::Response {
        self.client
            .head(format!("{}{}", self.base_url, path))
            .send()
            .await
            .expect("HEAD request failed")
    }

    /// Send an OPTIONS request
    pub async fn options(&self, path: &str) -> reqwest::Response {
        self.client
            .request(reqwest::Method::OPTIONS, format!("{}{}", self.base_url, path))
            .send()
            .await
            .expect("OPTIONS request failed")
    }

    /// Send a TRACE request
    pub async fn trace(&self, path: &str) -> reqwest::Response {
        self.client
            .request(reqwest::Method::TRACE, format!("{}{}", self.base_url, path))
            .send()
            .await
            .expect("TRACE request failed")
    }

    /// Send a custom request with any HTTP method
    pub async fn request(&self, method: reqwest::Method, path: &str) -> reqwest::Response {
        self.client
            .request(method, format!("{}{}", self.base_url, path))
            .send()
            .await
            .expect("Custom request failed")
    }

    /// Get the underlying reqwest Client for custom requests
    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl std::fmt::Debug for TestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestClient")
            .field("base_url", &self.base_url)
            .finish()
    }
}
