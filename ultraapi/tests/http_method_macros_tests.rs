use std::collections::HashMap;
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct UpdateUser {
    name: String,
}

// ---- PATCH Method Test ----

#[patch("/users/{id}")]
async fn patch_user(id: i64, body: UpdateUser) -> User {
    User {
        id,
        name: body.name,
    }
}

#[tokio::test]
async fn test_patch_macro_returns_200() {
    let app = UltraApiApp::new();
    let client = TestClient::new(app).await;

    let response = client
        .patch(
            "/users/1",
            &UpdateUser {
                name: "Updated".to_string(),
            },
        )
        .await;

    assert_eq!(response.status(), 200);

    let user: User = response.json().await.unwrap();
    assert_eq!(user.id, 1);
    assert_eq!(user.name, "Updated");
}

#[test]
fn test_patch_macro_in_openapi() {
    // Get the OpenAPI spec by building an app and accessing its spec
    let app = UltraApiApp::new().title("Test API").version("1.0.0");

    // Build the router which generates the spec
    let _router = app.into_router();

    // Access the spec from inventory
    let mut paths = HashMap::new();
    for route in ultraapi::inventory::iter::<&ultraapi::RouteInfo> {
        if route.path == "/users/{id}" {
            assert_eq!(route.method, "PATCH", "Method should be PATCH");
        }
        paths.insert(route.path.to_string(), route.method.to_string());
    }

    assert_eq!(paths.get("/users/{id}"), Some(&"PATCH".to_string()));
}

// ---- HEAD Method Test ----

#[head("/head-test")]
async fn head_test() -> String {
    "head response".to_string()
}

#[tokio::test]
async fn test_head_macro_returns_200() {
    let app = UltraApiApp::new();
    let client = TestClient::new(app).await;

    let response = client.head("/head-test").await;

    assert_eq!(response.status(), 200);
}

#[test]
fn test_head_macro_in_openapi() {
    let app = UltraApiApp::new().title("Test API").version("1.0.0");

    let _router = app.into_router();

    let mut paths = HashMap::new();
    for route in ultraapi::inventory::iter::<&ultraapi::RouteInfo> {
        if route.path == "/head-test" {
            assert_eq!(route.method, "HEAD", "Method should be HEAD");
        }
        paths.insert(route.path.to_string(), route.method.to_string());
    }

    assert_eq!(paths.get("/head-test"), Some(&"HEAD".to_string()));
}

// ---- OPTIONS Method Test ----

#[options("/options-test")]
async fn options_test() -> String {
    "options response".to_string()
}

#[tokio::test]
async fn test_options_macro_returns_200() {
    let app = UltraApiApp::new();
    let client = TestClient::new(app).await;

    let response = client.options("/options-test").await;

    assert_eq!(response.status(), 200);
}

#[test]
fn test_options_macro_in_openapi() {
    let app = UltraApiApp::new().title("Test API").version("1.0.0");

    let _router = app.into_router();

    let mut paths = HashMap::new();
    for route in ultraapi::inventory::iter::<&ultraapi::RouteInfo> {
        if route.path == "/options-test" {
            assert_eq!(route.method, "OPTIONS", "Method should be OPTIONS");
        }
        paths.insert(route.path.to_string(), route.method.to_string());
    }

    assert_eq!(paths.get("/options-test"), Some(&"OPTIONS".to_string()));
}

// ---- TRACE Method Test ----

#[trace("/trace-test")]
async fn trace_test() -> String {
    "trace response".to_string()
}

#[tokio::test]
async fn test_trace_macro_returns_200() {
    let app = UltraApiApp::new();
    let client = TestClient::new(app).await;

    let response = client.trace("/trace-test").await;

    assert_eq!(response.status(), 200);
}

#[test]
fn test_trace_macro_in_openapi() {
    let app = UltraApiApp::new().title("Test API").version("1.0.0");

    let _router = app.into_router();

    let mut paths = HashMap::new();
    for route in ultraapi::inventory::iter::<&ultraapi::RouteInfo> {
        if route.path == "/trace-test" {
            assert_eq!(route.method, "TRACE", "Method should be TRACE");
        }
        paths.insert(route.path.to_string(), route.method.to_string());
    }

    assert_eq!(paths.get("/trace-test"), Some(&"TRACE".to_string()));
}

// ---- All HTTP Methods Test ----

#[get("/all-methods/get")]
async fn all_methods_get() -> User {
    User {
        id: 1,
        name: "get".to_string(),
    }
}

#[post("/all-methods/post")]
async fn all_methods_post(body: User) -> User {
    body
}

#[put("/all-methods/put")]
async fn all_methods_put(body: User) -> User {
    body
}

#[delete("/all-methods/delete")]
async fn all_methods_delete() {}

#[patch("/all-methods/patch")]
async fn all_methods_patch(body: UpdateUser) -> User {
    User {
        id: 1,
        name: body.name,
    }
}

#[head("/all-methods/head")]
async fn all_methods_head() -> String {
    "head".to_string()
}

#[options("/all-methods/options")]
async fn all_methods_options() -> String {
    "options".to_string()
}

#[trace("/all-methods/trace")]
async fn all_methods_trace() -> String {
    "trace".to_string()
}

#[tokio::test]
async fn test_all_http_methods_runtime() {
    let app = UltraApiApp::new();
    let client = TestClient::new(app).await;

    // Test GET
    let response = client.get("/all-methods/get").await;
    assert_eq!(response.status(), 200);

    // Test POST
    let response = client
        .post(
            "/all-methods/post",
            &User {
                id: 1,
                name: "test".to_string(),
            },
        )
        .await;
    assert_eq!(response.status(), 201);

    // Test PUT
    let response = client
        .put(
            "/all-methods/put",
            &User {
                id: 1,
                name: "test".to_string(),
            },
        )
        .await;
    assert_eq!(response.status(), 200);

    // Test DELETE
    let response = client.delete("/all-methods/delete").await;
    assert_eq!(response.status(), 204);

    // Test PATCH
    let response = client
        .patch(
            "/all-methods/patch",
            &UpdateUser {
                name: "test".to_string(),
            },
        )
        .await;
    assert_eq!(response.status(), 200);

    // Test HEAD
    let response = client.head("/all-methods/head").await;
    assert_eq!(response.status(), 200);

    // Test OPTIONS
    let response = client.options("/all-methods/options").await;
    assert_eq!(response.status(), 200);

    // Test TRACE
    let response = client.trace("/all-methods/trace").await;
    assert_eq!(response.status(), 200);
}

#[test]
fn test_all_eight_http_methods_in_route_info() {
    let app = UltraApiApp::new().title("Test API").version("1.0.0");

    let _router = app.into_router();

    // Collect all methods from route info
    let mut methods: Vec<&str> = Vec::new();
    for route in ultraapi::inventory::iter::<&ultraapi::RouteInfo> {
        if route.path.starts_with("/all-methods/") {
            methods.push(route.method);
        }
    }

    // Sort and deduplicate
    methods.sort();
    methods.dedup();

    // Should have all 8 HTTP methods
    assert!(methods.contains(&"GET"), "Should contain GET");
    assert!(methods.contains(&"POST"), "Should contain POST");
    assert!(methods.contains(&"PUT"), "Should contain PUT");
    assert!(methods.contains(&"DELETE"), "Should contain DELETE");
    assert!(methods.contains(&"PATCH"), "Should contain PATCH");
    assert!(methods.contains(&"HEAD"), "Should contain HEAD");
    assert!(methods.contains(&"OPTIONS"), "Should contain OPTIONS");
    assert!(methods.contains(&"TRACE"), "Should contain TRACE");

    assert_eq!(methods.len(), 8, "Should have exactly 8 methods");
}
