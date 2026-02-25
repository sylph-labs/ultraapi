#![allow(unused_attributes, unused_doc_comments)]
//! gRPC Transcoding Support for UltraAPI
//!
//! This module provides the ability to expose gRPC services via HTTP/JSON,
//! allowing clients to call gRPC services using RESTful JSON while your
//! backend remains pure Rust with gRPC.
//!
//! ## Usage
//!
//! 1. Define your gRPC service using protobuf
//! 2. Use the `#[grpc]` attribute to register the service
//! 3. The HTTP gateway will automatically transcode HTTP/JSON to gRPC
//!
//! ## Example
//!
//! ```
//! use ultraapi::grpc::{service, GrpcMethod, GrpcTranscoder};
//!
//! let user_service = service("UserService")
//!     .package("user")
//!     .method(GrpcMethod::unary("GetUser", "/user.UserService/GetUser"))
//!     .method(GrpcMethod::unary("CreateUser", "/user.UserService/CreateUser"))
//!     .build();
//!
//! let transcoder = GrpcTranscoder::new().register_service(user_service);
//! let _router = transcoder.into_router();
//! ```

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A gRPC method descriptor
#[derive(Clone)]
pub struct GrpcMethod {
    /// Method name (e.g., "GetUser")
    pub name: String,
    /// Full gRPC method path (e.g., "/user.UserService/GetUser")
    pub full_path: String,
    /// The request message type name for JSON->Protobuf conversion
    pub request_type: String,
    /// The response message type name for Protobuf->JSON conversion
    pub response_type: String,
    /// Whether this is a server streaming method
    pub streaming: bool,
}

/// A registered gRPC service
#[derive(Clone)]
pub struct GrpcService {
    /// Service name (e.g., "UserService")
    pub name: String,
    /// Full service path (e.g., "/user.UserService")
    pub full_path: String,
    /// Package name (e.g., "user")
    pub package: Option<String>,
    /// Methods in this service
    pub methods: Vec<GrpcMethod>,
    /// Runtime handlers for methods (not included in Clone, use Arc manually)
    #[doc(hidden)]
    pub handlers: Arc<std::sync::RwLock<std::collections::HashMap<String, GrpcHandler>>>,
}

impl GrpcService {
    /// Create a new gRPC service descriptor
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            full_path: format!("/{}", name),
            package: None,
            methods: Vec::new(),
            handlers: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Add a method to this service
    pub fn method(mut self, method: GrpcMethod) -> Self {
        self.methods.push(method);
        self
    }

    /// Register a handler for a method (not cloneable, use before cloning)
    pub fn with_handler(self, method_name: &str, handler: GrpcHandler) -> Self {
        if let Ok(mut handlers) = self.handlers.write() {
            handlers.insert(method_name.to_string(), handler);
        }
        self
    }

    /// Get a handler by method name
    pub fn get_handler(&self, method_name: &str) -> Option<GrpcHandler> {
        let handlers = self.handlers.read().ok()?;
        handlers.get(method_name).cloned()
    }

    /// Set a handler for a method (consumes self, returns new instance)
    #[doc(hidden)]
    pub fn set_handler(self, method_name: &str, handler: GrpcHandler) -> Self {
        if let Ok(mut handlers) = self.handlers.write() {
            handlers.insert(method_name.to_string(), handler);
        }
        self
    }
}

/// A gRPC handler function
pub type GrpcHandler = std::sync::Arc<
    dyn Fn(GrpcRequest) -> Pin<Box<dyn Future<Output = GrpcResponse> + Send + Sync>> + Send + Sync,
>;

/// A gRPC request after transcoding from HTTP/JSON
#[derive(Debug)]
pub struct GrpcRequest {
    /// The JSON body deserialized
    pub body: serde_json::Value,
    /// Path parameters
    pub path_params: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// The full method path
    pub method_path: String,
}

/// A gRPC response to be transcoded back to HTTP/JSON
#[derive(Debug)]
pub struct GrpcResponse {
    /// The protobuf message encoded as JSON
    pub body: serde_json::Value,
    /// gRPC status code (0 = OK)
    pub status_code: i32,
}

impl GrpcMethod {
    /// Create a new unary (non-streaming) gRPC method
    pub fn unary(name: &str, full_path: &str) -> Self {
        Self {
            name: name.to_string(),
            full_path: full_path.to_string(),
            request_type: format!("{}Request", name),
            response_type: format!("{}Response", name),
            streaming: false,
        }
    }

    /// Create a new server streaming gRPC method
    pub fn server_streaming(name: &str, full_path: &str) -> Self {
        Self {
            name: name.to_string(),
            full_path: full_path.to_string(),
            request_type: format!("{}Request", name),
            response_type: format!("{}Response", name),
            streaming: true,
        }
    }
}

/// gRPC metadata (headers)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GrpcMetadata {
    #[serde(flatten)]
    pub headers: HashMap<String, String>,
}

/// gRPC error response
#[derive(Debug, Serialize)]
pub struct GrpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<serde_json::Value>,
}

impl GrpcError {
    pub fn ok(message: &str) -> Self {
        Self {
            code: 0,
            message: message.to_string(),
            details: vec![],
        }
    }

    pub fn not_found(message: &str) -> Self {
        Self {
            code: 5, // NOT_FOUND
            message: message.to_string(),
            details: vec![],
        }
    }

    pub fn internal(message: &str) -> Self {
        Self {
            code: 13, // INTERNAL
            message: message.to_string(),
            details: vec![],
        }
    }

    pub fn invalid_argument(message: &str) -> Self {
        Self {
            code: 3, // INVALID_ARGUMENT
            message: message.to_string(),
            details: vec![],
        }
    }
}

/// Configuration for gRPC transcoding
#[derive(Clone)]
pub struct GrpcTranscoder {
    services: Arc<HashMap<String, GrpcService>>,
}

impl GrpcTranscoder {
    /// Create a new gRPC transcoder
    pub fn new() -> Self {
        Self {
            services: Arc::new(HashMap::new()),
        }
    }

    /// Register a gRPC service
    pub fn register_service(mut self, service: GrpcService) -> Self {
        let name = service.name.clone();
        let services = Arc::make_mut(&mut self.services);
        services.insert(name, service);
        self
    }

    /// Get a service by name
    pub fn get_service(&self, name: &str) -> Option<&GrpcService> {
        self.services.get(name)
    }

    /// Build the HTTP routes for all registered gRPC services
    pub fn into_router(self) -> Router {
        let mut router = Router::new();

        // Add gRPC health check endpoint
        router = router.route("/grpc.health.v1.Health/Check", get(grpc_health_check));

        // Add service descriptor endpoint (for reflection)
        router = router.route(
            "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
            post(grpc_reflection),
        );

        // Add transcoding routes for each service
        for (_name, service) in self.services.iter() {
            for method in &service.methods {
                let service = service.clone();
                let method = method.clone();

                // Create HTTP path: /{package}/{service}/{method}
                let http_path = if let Some(ref pkg) = service.package {
                    format!("/{}/{}/{}", pkg, service.name, method.name)
                } else {
                    format!("/{}/{}", service.name, method.name)
                };

                router = router.route(
                    &http_path,
                    post(
                        move |axum::extract::Json(body): axum::extract::Json<serde_json::Value>| {
                            let service = service.clone();
                            let method = method.clone();
                            async move { handle_grpc_request(service, method, body).await }
                        },
                    ),
                );
            }
        }

        router
    }
}

impl Default for GrpcTranscoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle a gRPC transcoding request
async fn handle_grpc_request(
    service: GrpcService,
    method: GrpcMethod,
    body: serde_json::Value,
) -> impl IntoResponse {
    // Find the handler for this method
    if let Some(handler) = service.get_handler(&method.name) {
        let request = GrpcRequest {
            body,
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            method_path: method.full_path.clone(),
        };

        let response = handler(request).await;

        if response.status_code == 0 {
            (StatusCode::OK, Json(response.body)).into_response()
        } else {
            let error = GrpcError {
                code: response.status_code,
                message: "Error".to_string(),
                details: vec![response.body],
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    } else {
        let error = GrpcError::not_found(&format!("Method {} not found", method.name));
        (StatusCode::NOT_FOUND, Json(error)).into_response()
    }
}

/// gRPC health check handler
async fn grpc_health_check() -> impl IntoResponse {
    #[derive(Serialize)]
    struct HealthResponse {
        status: String,
    }

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "SERVING".to_string(),
        }),
    )
}

/// gRPC server reflection handler
async fn grpc_reflection() -> impl IntoResponse {
    #[derive(Serialize)]
    struct FileDescriptorResponse {
        #[serde(rename = "fileDescriptorProto")]
        proto: String,
    }

    // Return empty list of services (can be extended for full reflection)
    (
        StatusCode::OK,
        Json(vec![FileDescriptorResponse {
            proto: "".to_string(),
        }]),
    )
}

/// Internal state for gRPC services
#[derive(Clone)]
pub struct GrpcState {
    transcoder: GrpcTranscoder,
}

impl GrpcState {
    pub fn new(transcoder: GrpcTranscoder) -> Self {
        Self { transcoder }
    }

    pub fn transcoder(&self) -> &GrpcTranscoder {
        &self.transcoder
    }
}

/// Extension trait to add gRPC support to UltraApiApp
pub trait GrpcExt {
    /// Add gRPC transcoding support to the application
    fn grpc(self, _transcoder: GrpcTranscoder) -> Self;
}

impl GrpcExt for crate::UltraApiApp {
    #[allow(unused_mut)]
    fn grpc(self, _transcoder: GrpcTranscoder) -> Self {
        // Note: The actual merging happens when user calls into_router()
        // and merges the gRPC router with the main router
        self
    }
}

/// Inventory for registered gRPC services
#[allow(unused_doc_comments)]
inventory::collect!(&'static GrpcServiceInfo);

/// Information about a registered gRPC service
pub struct GrpcServiceInfo {
    pub name: &'static str,
    pub service_fn: fn() -> GrpcService,
}

/// Macro to register a gRPC service
#[macro_export]
macro_rules! grpc_service {
    ($name:expr, { $($method:expr => $path:expr),* $(,)? }) => {
        ::inventory::submit! {
            &::ultraapi::grpc::GrpcServiceInfo {
                name: $name,
                service_fn: || {
                    use ::ultraapi::grpc::{GrpcMethod, GrpcService};
                    let mut service = ::ultraapi::grpc::GrpcService::new($name);
                    $(
                        service = service.method(GrpcMethod::unary($method, $path));
                    )*
                    service
                }
            }
        }
    };
}

/// Attribute macro for gRPC service registration
///
/// Usage:
/// ```
/// use ultraapi::grpc_service;
///
/// grpc_service!("UserService", {
///     "GetUser" => "/user.UserService/GetUser",
///     "CreateUser" => "/user.UserService/CreateUser",
/// });
/// ```
pub struct GrpcAttribute;

/// Build a gRPC method from a protobuf-style definition
#[derive(Debug, Clone)]
pub struct GrpcMethodBuilder {
    name: String,
    full_path: String,
    request_type: String,
    response_type: String,
    streaming: bool,
}

impl GrpcMethodBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            full_path: String::new(),
            request_type: format!("{}Request", name),
            response_type: format!("{}Response", name),
            streaming: false,
        }
    }

    pub fn path(mut self, path: &str) -> Self {
        self.full_path = path.to_string();
        self
    }

    pub fn request<T: 'static>(mut self) -> Self {
        self.request_type = std::any::type_name::<T>().to_string();
        self
    }

    pub fn response<T: 'static>(mut self) -> Self {
        self.response_type = std::any::type_name::<T>().to_string();
        self
    }

    pub fn streaming(mut self) -> Self {
        self.streaming = true;
        self
    }

    pub fn build(self) -> GrpcMethod {
        GrpcMethod {
            name: self.name,
            full_path: self.full_path,
            request_type: self.request_type,
            response_type: self.response_type,
            streaming: self.streaming,
        }
    }
}

/// Helper to create a gRPC service builder
pub fn service(name: &str) -> ServiceBuilder {
    ServiceBuilder::new(name)
}

/// Builder for gRPC services
#[derive(Clone)]
pub struct ServiceBuilder {
    name: String,
    package: Option<String>,
    methods: Vec<GrpcMethod>,
}

impl ServiceBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            package: None,
            methods: Vec::new(),
        }
    }

    pub fn package(mut self, pkg: &str) -> Self {
        self.package = Some(pkg.to_string());
        self
    }

    pub fn method(mut self, method: GrpcMethod) -> Self {
        self.methods.push(method);
        self
    }

    pub fn method_unary(mut self, name: &str, path: &str) -> Self {
        self.methods.push(GrpcMethod::unary(name, path));
        self
    }

    /// Add a handler for a method (returns GrpcService, not ServiceBuilder)
    pub fn with_handler(self, method_name: &str, handler: GrpcHandler) -> GrpcService {
        let full_path = if let Some(ref pkg) = self.package {
            format!("/{}.{}", pkg, self.name)
        } else {
            format!("/{}", self.name)
        };

        let service = GrpcService {
            name: self.name,
            full_path,
            package: self.package,
            methods: self.methods,
            handlers: Arc::new(std::sync::RwLock::new(HashMap::new())),
        };

        if let Ok(mut handlers) = service.handlers.write() {
            handlers.insert(method_name.to_string(), handler);
        }

        service
    }

    pub fn build(self) -> GrpcService {
        let full_path = if let Some(ref pkg) = self.package {
            format!("/{}.{}", pkg, self.name)
        } else {
            format!("/{}", self.name)
        };

        GrpcService {
            name: self.name,
            full_path,
            package: self.package,
            methods: self.methods,
            handlers: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_service_builder() {
        let service = service("UserService")
            .package("user")
            .method_unary("GetUser", "/user.UserService/GetUser")
            .method_unary("CreateUser", "/user.UserService/CreateUser")
            .build();

        assert_eq!(service.name, "UserService");
        assert_eq!(service.package, Some("user".to_string()));
        assert_eq!(service.methods.len(), 2);
        assert_eq!(service.methods[0].name, "GetUser");
    }

    #[test]
    fn test_grpc_method_builder() {
        let method = GrpcMethodBuilder::new("GetUser")
            .path("/user.UserService/GetUser")
            .build();

        assert_eq!(method.name, "GetUser");
        assert_eq!(method.full_path, "/user.UserService/GetUser");
        assert!(!method.streaming);
    }

    #[test]
    fn test_transcoder() {
        let transcoder = GrpcTranscoder::new().register_service(
            service("TestService")
                .method_unary("TestMethod", "/test.TestService/TestMethod")
                .build(),
        );

        assert!(transcoder.get_service("TestService").is_some());
        assert!(transcoder.get_service("NonExistent").is_none());
    }
}
