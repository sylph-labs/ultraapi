//! TestClient - Built-in test client for UltraAPI applications
//! 
//! This module provides a TestClient similar to FastAPI's TestClient,
//! allowing you to test your UltraAPI applications without running an actual server.
//! 
//! The TestClient automatically runs startup/shutdown lifecycle hooks.

use crate::lifespan::LifespanRunner;
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
    /// Optional lifespan runner for shutdown
    lifespan_runner: Option<LifespanRunner>,
}

impl TestClient {
    /// Create a new TestClient from an UltraApiApp (async)
    /// 
    /// This converts the app to a Router and starts a test server.
    /// Lifecycle hooks (startup/shutdown) are automatically managed.
    /// 
    /// # Arguments
    /// 
    /// * `app` - An UltraApiApp instance
    /// 
    /// # Returns
    /// 
    /// A TestClient instance connected to a running test server
    pub async fn new(app: UltraApiApp) -> Self {
        let (router, runner) = app.into_router_with_lifespan();

        // Run startup hooks before starting the test server.
        runner.ensure_startup().await;

        Self::new_with_lifespan(router, runner).await
    }

    /// Create a new TestClient from a Router with an existing LifespanRunner
    ///
    /// The server will run until the TestClient is dropped or shutdown() is called.
    pub async fn new_with_lifespan(router: Router, runner: LifespanRunner) -> Self {
        // Create a TCP listener on a random port
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind to random port");
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        // Spawn the server with graceful shutdown.
        // The server stops when `runner.shutdown()` is called.
        let runner_for_server = runner.clone();
        let server_handle = spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    runner_for_server.wait_for_shutdown().await;
                })
                .await
                .expect("Server error");
        });

        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Create the reqwest client
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            base_url,
            _server_handle: server_handle,
            lifespan_runner: Some(runner),
        }
    }

    /// Create a new TestClient from a Router (async)
    /// 
    /// Note: This does NOT run lifecycle hooks. Use `TestClient::new()` with
    /// `UltraApiApp` if you need lifecycle hooks to run.
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

        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Create the reqwest client
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            base_url,
            _server_handle: server_handle,
            lifespan_runner: None,
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

    /// Trigger shutdown hooks (if lifecycle is enabled)
    /// 
    /// This must be called to ensure shutdown hooks run.
    /// The Drop implementation also triggers shutdown, but calling this
    /// explicitly allows you to verify shutdown behavior.
    pub async fn shutdown(&self) {
        if let Some(ref runner) = self.lifespan_runner {
            runner.shutdown().await;
        }
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        // Trigger shutdown in the background when dropped
        if let Some(runner) = self.lifespan_runner.clone() {
            tokio::spawn(async move {
                runner.shutdown().await;
            });
        }
    }
}

impl std::fmt::Debug for TestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestClient")
            .field("base_url", &self.base_url)
            .finish()
    }
}
