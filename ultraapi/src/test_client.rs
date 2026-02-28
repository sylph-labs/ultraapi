//! TestClient - Built-in test client for UltraAPI applications
//!
//! This module provides a TestClient similar to FastAPI's TestClient,
//! allowing you to test your UltraAPI applications without running an actual server.
//!
//! The TestClient automatically runs startup/shutdown lifecycle hooks.
//!
//! ## Two Execution Modes
//!
//! ### Network Mode (Default)
//!
//! The default `TestClient::new()` starts an actual HTTP server on a random port
//! and uses reqwest to make requests. This is useful for testing the full HTTP stack.
//!
//! ### In-Process Mode
//!
//! The in-process mode uses `tower::ServiceExt::oneshot` to directly call the Router
//! without starting a network server. This provides:
//!
//! - **Faster execution**: No network overhead or server spawning
//! - **Better control**: Direct access to the Router for testing
//! - **No port conflicts**: No need to bind to network ports
//!
//! # Example
//!
//! ```ignore
//! use ultraapi::prelude::*;
//!
//! #[get("/hello")]
//! async fn hello() -> String {
//!     "Hello, World!".to_string()
//! }
//!
//! #[tokio::test]
//! async fn test_hello_in_process() {
//!     let app = UltraApiApp::new();
//!     // Use in-process mode for faster tests
//!     let client = TestClient::new_in_process(app);
//!     
//!     let response = client.get("/hello").await;
//!     assert_eq!(response.status(), 200);
//!     
//!     let body = response.text().await.unwrap();
//!     assert_eq!(body, "Hello, World!");
//! }
//! ```

use crate::lifespan::LifespanRunner;
use crate::UltraApiApp;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use reqwest::Client;
use tokio::net::TcpListener;
use tokio::spawn;
use tower::ServiceExt;

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
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to random port");
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
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to random port");
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

    /// Create a new **in-process** test client from an UltraApiApp (async).
    ///
    /// This does **not** bind a TCP port and does **not** use reqwest.
    /// Requests are executed by calling the axum Router (tower::Service) directly.
    pub async fn new_in_process(app: UltraApiApp) -> InProcessTestClient {
        InProcessTestClient::new_in_process_async(app).await
    }

    /// Create a new **in-process** test client from a Router.
    ///
    /// Note: This does NOT run lifecycle hooks.
    pub fn new_router_in_process(router: Router) -> InProcessTestClient {
        InProcessTestClient::new_router_in_process(router)
    }

    /// Create a new **in-process** test client from a Router with LifespanRunner.
    pub async fn new_router_in_process_with_lifespan(
        router: Router,
        runner: LifespanRunner,
    ) -> InProcessTestClient {
        InProcessTestClient::new_router_in_process_with_lifespan(router, runner).await
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
            .request(
                reqwest::Method::OPTIONS,
                format!("{}{}", self.base_url, path),
            )
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

// ============================================================================
// In-Process Test Client (tower::Service 直叩き)
// ============================================================================

/// Response type for in-process test client
///
/// Provides a convenient API similar to reqwest::Response for extracting
/// response data (status, headers, body).
pub struct TestResponse {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: bytes::Bytes,
}

impl TestResponse {
    /// Get the HTTP status code
    pub fn status(&self) -> u16 {
        self.status.as_u16()
    }

    /// Get the HTTP status
    pub fn status_code(&self) -> StatusCode {
        self.status
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Get all headers
    pub fn headers(&self) -> &axum::http::HeaderMap {
        &self.headers
    }

    /// Get the response body as bytes
    pub fn bytes(&self) -> &bytes::Bytes {
        &self.body
    }

    /// Get the response body as text (UTF-8)
    pub fn text(&self) -> Result<String, std::io::Error> {
        String::from_utf8(self.body.to_vec())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Get the response body as JSON
    pub async fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }
}

impl std::fmt::Debug for TestResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestResponse")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body_length", &self.body.len())
            .finish()
    }
}

/// In-process test client using tower::ServiceExt::oneshot
///
/// This client directly calls the Router without starting a network server,
/// providing faster tests with better control.
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
/// async fn test_in_process() {
///     let app = UltraApiApp::new();
///     let client = TestClient::new_in_process(app);
///     
///     let response = client.get("/hello").await;
///     assert_eq!(response.status(), 200);
///     assert_eq!(response.text().unwrap(), "Hello, World!");
/// }
/// ```
pub struct InProcessTestClient {
    router: Router,
    /// Optional lifespan runner for shutdown
    lifespan_runner: Option<LifespanRunner>,
}

impl InProcessTestClient {
    /// Create a new in-process TestClient from an UltraApiApp
    ///
    /// This converts the app to a Router and runs lifecycle hooks.
    ///
    /// # Arguments
    ///
    /// * `app` - An UltraApiApp instance
    ///
    /// # Returns
    ///
    /// An InProcessTestClient instance
    pub fn new_in_process(mut app: UltraApiApp) -> Self {
        let (router, runner) = app.into_router_with_lifespan();

        // Run startup hooks synchronously
        let runtime = tokio::runtime::Handle::current();
        runtime.block_on(async {
            runner.ensure_startup().await;
        });

        Self {
            router,
            lifespan_runner: Some(runner),
        }
    }

    /// Create a new in-process TestClient from an UltraApiApp (async version)
    ///
    /// Use this when you're already in an async context.
    ///
    /// # Arguments
    ///
    /// * `app` - An UltraApiApp instance
    ///
    /// # Returns
    ///
    /// An InProcessTestClient instance
    pub async fn new_in_process_async(app: UltraApiApp) -> Self {
        let (router, runner) = app.into_router_with_lifespan();

        // Run startup hooks
        runner.ensure_startup().await;

        Self {
            router,
            lifespan_runner: Some(runner),
        }
    }

    /// Create a new in-process TestClient from a Router
    ///
    /// Note: This does NOT run lifecycle hooks. Use `new_in_process()` with
    /// `UltraApiApp` if you need lifecycle hooks to run.
    ///
    /// # Arguments
    ///
    /// * `router` - An axum Router instance
    ///
    /// # Returns
    ///
    /// An InProcessTestClient instance
    pub fn new_router_in_process(router: Router) -> Self {
        Self {
            router,
            lifespan_runner: None,
        }
    }

    /// Create a new in-process TestClient from a Router with LifespanRunner
    ///
    /// # Arguments
    ///
    /// * `router` - An axum Router instance
    /// * `runner` - A LifespanRunner for lifecycle management
    ///
    /// # Returns
    ///
    /// An InProcessTestClient instance
    pub async fn new_router_in_process_with_lifespan(
        router: Router,
        runner: LifespanRunner,
    ) -> Self {
        runner.ensure_startup().await;

        Self {
            router,
            lifespan_runner: Some(runner),
        }
    }

    /// Send a GET request
    pub async fn get(&self, path: &str) -> TestResponse {
        self.request(axum::http::Method::GET, path, None).await
    }

    /// Send a POST request with JSON body
    pub async fn post<B: serde::Serialize>(&self, path: &str, body: &B) -> TestResponse {
        let body_bytes = serde_json::to_vec(body).expect("Failed to serialize body");
        self.request_with_header(
            axum::http::Method::POST,
            path,
            Some(body_bytes.into()),
            Some(("content-type", "application/json")),
        )
        .await
    }

    /// Send a POST request with raw body
    pub async fn post_raw(&self, path: &str, body: impl Into<bytes::Bytes>) -> TestResponse {
        self.request(axum::http::Method::POST, path, Some(body.into()))
            .await
    }

    /// Send a PUT request with JSON body
    pub async fn put<B: serde::Serialize>(&self, path: &str, body: &B) -> TestResponse {
        let body_bytes = serde_json::to_vec(body).expect("Failed to serialize body");
        self.request_with_header(
            axum::http::Method::PUT,
            path,
            Some(body_bytes.into()),
            Some(("content-type", "application/json")),
        )
        .await
    }

    /// Send a PUT request with raw body
    pub async fn put_raw(&self, path: &str, body: impl Into<bytes::Bytes>) -> TestResponse {
        self.request(axum::http::Method::PUT, path, Some(body.into()))
            .await
    }

    /// Send a DELETE request
    pub async fn delete(&self, path: &str) -> TestResponse {
        self.request(axum::http::Method::DELETE, path, None).await
    }

    /// Send a PATCH request with JSON body
    pub async fn patch<B: serde::Serialize>(&self, path: &str, body: &B) -> TestResponse {
        let body_bytes = serde_json::to_vec(body).expect("Failed to serialize body");
        self.request_with_header(
            axum::http::Method::PATCH,
            path,
            Some(body_bytes.into()),
            Some(("content-type", "application/json")),
        )
        .await
    }

    /// Send a HEAD request
    pub async fn head(&self, path: &str) -> TestResponse {
        self.request(axum::http::Method::HEAD, path, None).await
    }

    /// Send an OPTIONS request
    pub async fn options(&self, path: &str) -> TestResponse {
        self.request(axum::http::Method::OPTIONS, path, None).await
    }

    /// Send a custom request with any HTTP method
    pub async fn request(
        &self,
        method: axum::http::Method,
        path: &str,
        body: Option<bytes::Bytes>,
    ) -> TestResponse {
        self.request_with_header(method, path, body, None).await
    }

    /// Send a request with custom headers
    pub async fn request_with_header(
        &self,
        method: axum::http::Method,
        path: &str,
        body: Option<bytes::Bytes>,
        header: Option<(&str, &str)>,
    ) -> TestResponse {
        let uri = path.to_string();

        let mut request_builder = Request::builder().uri(&uri).method(method);

        if let Some((key, value)) = header {
            request_builder = request_builder.header(key, value);
        }

        let body = body.unwrap_or_default();
        let request = request_builder
            .body(Body::from(body))
            .expect("Failed to build request");

        // Use tower::ServiceExt::oneshot to call the router directly
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("Request failed");

        // Extract status, headers, and body
        let status = response.status();
        let headers = response.headers().clone();

        // Collect the body
        let body = Body::from(response.into_body());
        let body_bytes = to_bytes(body, usize::MAX)
            .await
            .expect("Failed to read response body")
            .to_vec()
            .into();

        TestResponse {
            status,
            headers,
            body: body_bytes,
        }
    }

    /// Add custom headers to requests
    ///
    /// Returns a new client with the specified default headers.
    pub fn with_header(&self, key: &str, value: &str) -> InProcessTestClientWithHeader {
        InProcessTestClientWithHeader {
            router: self.router.clone(),
            lifespan_runner: self.lifespan_runner.clone(),
            default_headers: vec![(key.to_string(), value.to_string())],
        }
    }

    /// Trigger shutdown hooks (if lifecycle is enabled)
    pub async fn shutdown(&self) {
        if let Some(ref runner) = self.lifespan_runner {
            runner.shutdown().await;
        }
    }
}

impl Drop for InProcessTestClient {
    fn drop(&mut self) {
        // Trigger shutdown in the background when dropped
        if let Some(runner) = self.lifespan_runner.clone() {
            tokio::spawn(async move {
                runner.shutdown().await;
            });
        }
    }
}

impl std::fmt::Debug for InProcessTestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessTestClient").finish()
    }
}

/// In-process test client with default headers
pub struct InProcessTestClientWithHeader {
    router: Router,
    lifespan_runner: Option<LifespanRunner>,
    default_headers: Vec<(String, String)>,
}

impl InProcessTestClientWithHeader {
    /// Send a GET request with default headers
    pub async fn get(&self, path: &str) -> TestResponse {
        self.request(axum::http::Method::GET, path, None).await
    }

    /// Send a POST request with JSON body and default headers
    pub async fn post<B: serde::Serialize>(&self, path: &str, body: &B) -> TestResponse {
        let body_bytes = serde_json::to_vec(body).expect("Failed to serialize body");
        self.request_with_header(
            axum::http::Method::POST,
            path,
            Some(body_bytes.into()),
            Some(("content-type", "application/json")),
        )
        .await
    }

    /// Send a request with custom headers merged with default headers
    pub async fn request(
        &self,
        method: axum::http::Method,
        path: &str,
        body: Option<bytes::Bytes>,
    ) -> TestResponse {
        self.request_with_header(method, path, body, None).await
    }

    /// Send a request with custom headers
    pub async fn request_with_header(
        &self,
        method: axum::http::Method,
        path: &str,
        body: Option<bytes::Bytes>,
        header: Option<(&str, &str)>,
    ) -> TestResponse {
        let uri = path.to_string();

        let mut request_builder = Request::builder().uri(&uri).method(method);

        // Add default headers
        for (key, value) in &self.default_headers {
            request_builder = request_builder.header(key.as_str(), value.as_str());
        }

        // Add custom header if provided
        if let Some((key, value)) = header {
            request_builder = request_builder.header(key, value);
        }

        let body = body.unwrap_or_default();
        let request = request_builder
            .body(Body::from(body))
            .expect("Failed to build request");

        // Use tower::ServiceExt::oneshot to call the router directly
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("Request failed");

        // Extract status, headers, and body
        let status = response.status();
        let headers = response.headers().clone();

        // Collect the body
        let body = Body::from(response.into_body());
        let body_bytes = to_bytes(body, usize::MAX)
            .await
            .expect("Failed to read response body")
            .to_vec()
            .into();

        TestResponse {
            status,
            headers,
            body: body_bytes,
        }
    }
}

impl std::fmt::Debug for InProcessTestClientWithHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessTestClientWithHeader")
            .field("default_headers", &self.default_headers)
            .finish()
    }
}
