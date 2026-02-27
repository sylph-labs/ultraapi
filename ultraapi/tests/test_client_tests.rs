//! TestClient tests
//! 
//! Tests for the TestClient functionality.

use ultraapi::prelude::*;

// --- Test Models ---

#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct CreateUserRequest {
    #[validate(min_length = 1, max_length = 100)]
    name: String,
    
    #[validate(email)]
    email: String,
}

// --- Test Routes ---

#[get("/users/{id}")]
async fn get_user(id: i64) -> Result<User, ApiError> {
    if id <= 0 {
        return Err(ApiError::bad_request("Invalid user ID".to_string()));
    }
    Ok(User {
        id,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    })
}

#[post("/users")]
#[status(201)]
async fn create_user(body: CreateUserRequest) -> User {
    User {
        id: 42,
        name: body.name,
        email: body.email,
    }
}

#[get("/users")]
async fn list_users() -> Vec<User> {
    vec![
        User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
        User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
    ]
}

#[put("/users/{id}")]
async fn update_user(id: i64, body: CreateUserRequest) -> User {
    User {
        id,
        name: body.name,
        email: body.email,
    }
}

#[delete("/users/{id}")]
#[status(204)]
async fn delete_user(id: i64) -> Result<(), ApiError> {
    if id == 0 {
        return Err(ApiError::bad_request("Invalid user ID".to_string()));
    }
    Ok(())
}

#[get("/hello")]
async fn hello() -> String {
    "Hello, World!".to_string()
}

#[get("/json")]
async fn json_response() -> serde_json::Value {
    serde_json::json!({
        "message": "success",
        "data": [1, 2, 3]
    })
}

// --- Helper ---

fn create_test_app() -> UltraApiApp {
    UltraApiApp::new()
        .title("Test API")
        .version("0.1.0")
}

// --- Tests ---

#[tokio::test]
async fn test_client_get_request() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let response = client.get("/hello").await;
    assert_eq!(response.status(), 200);
    
    let body: String = response.json().await.unwrap();
    assert_eq!(body, "Hello, World!");
}

#[tokio::test]
async fn test_client_get_with_path_param() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let response = client.get("/users/42").await;
    assert_eq!(response.status(), 200);
    
    let user: User = response.json().await.unwrap();
    assert_eq!(user.id, 42);
    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
}

#[tokio::test]
async fn test_client_post_request() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let request = CreateUserRequest {
        name: "Charlie".to_string(),
        email: "charlie@example.com".to_string(),
    };
    
    let response = client.post("/users", &request).await;
    assert_eq!(response.status(), 201);
    
    let user: User = response.json().await.unwrap();
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.email, "charlie@example.com");
}

#[tokio::test]
async fn test_client_put_request() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let request = CreateUserRequest {
        name: "Updated".to_string(),
        email: "updated@example.com".to_string(),
    };
    
    let response = client.put("/users/1", &request).await;
    assert_eq!(response.status(), 200);
    
    let user: User = response.json().await.unwrap();
    assert_eq!(user.name, "Updated");
}

#[tokio::test]
async fn test_client_delete_request() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let response = client.delete("/users/1").await;
    assert_eq!(response.status(), 204);
}

#[tokio::test]
async fn test_client_json_response() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let response = client.get("/json").await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["message"], "success");
    assert_eq!(body["data"], serde_json::json!([1, 2, 3]));
}

#[tokio::test]
async fn test_client_multiple_requests() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    // First request
    let response1 = client.get("/hello").await;
    assert_eq!(response1.status(), 200);
    let body1: String = response1.json().await.unwrap();
    assert_eq!(body1, "Hello, World!");
    
    // Second request
    let response2 = client.get("/users/1").await;
    assert_eq!(response2.status(), 200);
    let user: User = response2.json().await.unwrap();
    assert_eq!(user.id, 1);
    
    // Third request - POST
    let request = CreateUserRequest {
        name: "Test".to_string(),
        email: "test@example.com".to_string(),
    };
    let response3 = client.post("/users", &request).await;
    assert_eq!(response3.status(), 201);
}

#[tokio::test]
async fn test_client_base_url() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let base_url = client.base_url();
    assert!(base_url.starts_with("http://"));
    assert!(base_url.contains(":"));
}

#[tokio::test]
async fn test_client_head_request() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let response = client.head("/hello").await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_client_error_response() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    // Test 404
    let response = client.get("/nonexistent").await;
    assert_eq!(response.status(), 404);
    
    // Test 400 (bad request)
    let response = client.get("/users/0").await;
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_client_get_underlying_client() {
    let app = create_test_app();
    let client = TestClient::new(app).await;
    
    let base_url = client.base_url();
    let reqwest_client = client.client();
    
    // Use the underlying client for custom requests
    let response = reqwest_client
        .get(format!("{}/hello", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
}
