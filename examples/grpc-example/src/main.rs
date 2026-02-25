//! gRPC Transcoding Example for UltraAPI
//!
//! This example demonstrates how to expose gRPC services via HTTP/JSON
//! using UltraAPI's gRPC transcoding support.
//!
//! Run with: cargo run --example grpc-example
//!
//! Then test with:
//! curl -X POST http://localhost:3002/user.UserService/GetUser \
//!   -H "Content-Type: application/json" \
//!   -d '{"id": 1}'
//!
//! curl -X POST http://localhost:3002/user.UserService/CreateUser \
//!   -H "Content-Type: application/json" \
//!   -d '{"name": "Alice", "email": "alice@example.com"}'

use std::sync::{Arc, Mutex};
use ultraapi::grpc::{service, GrpcHandler, GrpcMethod, GrpcRequest, GrpcResponse, GrpcTranscoder};
use ultraapi::prelude::*;

/// User model
#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
    status: String,
}

/// A simple in-memory user store
#[derive(Clone)]
struct UserStore {
    users: Arc<Mutex<Vec<User>>>,
}

impl UserStore {
    fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(vec![
                User {
                    id: 1,
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                    status: "active".to_string(),
                },
                User {
                    id: 2,
                    name: "Bob".to_string(),
                    email: "bob@example.com".to_string(),
                    status: "inactive".to_string(),
                },
            ])),
        }
    }

    fn get_user(&self, id: i64) -> Option<User> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == id)
            .cloned()
    }

    fn create_user(&self, name: String, email: String) -> User {
        let mut users = self.users.lock().unwrap();
        let id = users.len() as i64 + 1;
        let user = User {
            id,
            name,
            email,
            status: "active".to_string(),
        };
        users.push(user.clone());
        user
    }
}

/// Build the gRPC transcoder with our services
fn build_grpc_transcoder(store: UserStore) -> GrpcTranscoder {
    // Create handlers for our gRPC methods
    let get_user_handler: GrpcHandler = {
        let store = store.clone();
        Arc::new(move |req: GrpcRequest| {
            let store = store.clone();
            Box::pin(async move {
                // Extract user ID from request body
                let user_id = req.body.get("id").and_then(|v| v.as_i64()).unwrap_or(0);

                if let Some(user) = store.get_user(user_id) {
                    GrpcResponse {
                        body: serde_json::json!({
                            "user": user,
                            "message": "User found"
                        }),
                        status_code: 0,
                    }
                } else {
                    GrpcResponse {
                        body: serde_json::json!({
                            "user": serde_json::Value::Null,
                            "message": "User not found"
                        }),
                        status_code: 5, // NOT_FOUND
                    }
                }
            })
        })
    };

    let create_user_handler: GrpcHandler = {
        let store = store.clone();
        Arc::new(move |req: GrpcRequest| {
            let store = store.clone();
            Box::pin(async move {
                let name = req
                    .body
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let email = req
                    .body
                    .get("email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let user = store.create_user(name.clone(), email.clone());

                GrpcResponse {
                    body: serde_json::json!({
                        "user": user,
                        "message": "User created successfully"
                    }),
                    status_code: 0,
                }
            })
        })
    };

    // Build the gRPC service
    let user_service = service("UserService")
        .package("user")
        .method(GrpcMethod::unary("GetUser", "/user.UserService/GetUser"))
        .method(GrpcMethod::unary(
            "CreateUser",
            "/user.UserService/CreateUser",
        ))
        .with_handler("GetUser", get_user_handler)
        .with_handler("CreateUser", create_user_handler);

    // Create the transcoder and register the service
    GrpcTranscoder::new().register_service(user_service)
}

#[tokio::main]
async fn main() {
    // Create the user store
    let store = UserStore::new();

    // Build the gRPC transcoder
    let transcoder = build_grpc_transcoder(store);

    // Get the gRPC router
    let grpc_router = transcoder.into_router();

    // Use axum to serve the gRPC routes
    let addr = "0.0.0.0:3002";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    println!("🚀 gRPC Transcoding Example Server");
    println!("📖 Available endpoints:");
    println!("   POST /user.UserService/GetUser");
    println!("   POST /user.UserService/CreateUser");
    println!("   GET  /grpc.health.v1.Health/Check");
    println!("🌐 Server running at http://{}", addr);

    axum::serve(listener, grpc_router)
        .await
        .expect("Server error");
}
