use hayai::prelude::*;
use hayai::axum;
use serde_json::Value;

// ===== Auth validator =====

/// Simple Bearer token validator for testing
struct TestClaims {
    user_id: i64,
}

impl SecurityValidator for TestClaims {
    async fn validate(parts: &http::request::Parts) -> Result<Self, ApiError> {
        let header = parts.headers.get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("Missing Authorization header"))?;

        let token = header.strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::unauthorized("Invalid Bearer token format"))?;

        // Simulate token validation
        match token {
            "valid-token-42" => Ok(TestClaims { user_id: 42 }),
            "valid-token-99" => Ok(TestClaims { user_id: 99 }),
            _ => Err(ApiError::unauthorized("Invalid token")),
        }
    }
}

// ===== Models =====

#[api_model]
#[derive(Debug, Clone)]
struct UserProfile {
    id: i64,
    name: String,
}

// ===== Secured routes =====

/// Get the current user's profile (requires authentication)
#[get("/me")]
async fn get_me(auth: Auth<TestClaims>) -> UserProfile {
    UserProfile {
        id: auth.user_id,
        name: format!("User {}", auth.user_id),
    }
}

/// A public endpoint (no authentication required)
#[get("/health")]
async fn health_check() -> UserProfile {
    UserProfile { id: 0, name: "healthy".into() }
}

// ===== Test helpers =====

async fn spawn_auth_app() -> String {
    let router = hayai::HayaiRouter::new("/api")
        .route(get_me)
        .route(health_check);

    let app = hayai::HayaiApp::new()
        .title("Auth Test API")
        .version("0.1.0")
        .bearer_auth()
        .include(router)
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

// ===== E2E Tests: Runtime Auth Enforcement =====

#[tokio::test]
async fn test_secured_route_without_token_returns_401() {
    let base = spawn_auth_app().await;
    let resp = reqwest::get(format!("{base}/api/me")).await.unwrap();
    assert_eq!(resp.status(), 401);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Authorization"));
}

#[tokio::test]
async fn test_secured_route_with_invalid_token_returns_401() {
    let base = spawn_auth_app().await;
    let client = reqwest::Client::new();
    let resp = client.get(format!("{base}/api/me"))
        .header("Authorization", "Bearer bad-token")
        .send().await.unwrap();
    assert_eq!(resp.status(), 401);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Invalid token"));
}

#[tokio::test]
async fn test_secured_route_with_wrong_format_returns_401() {
    let base = spawn_auth_app().await;
    let client = reqwest::Client::new();
    let resp = client.get(format!("{base}/api/me"))
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .send().await.unwrap();
    assert_eq!(resp.status(), 401);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Bearer"));
}

#[tokio::test]
async fn test_secured_route_with_valid_token_returns_200() {
    let base = spawn_auth_app().await;
    let client = reqwest::Client::new();
    let resp = client.get(format!("{base}/api/me"))
        .header("Authorization", "Bearer valid-token-42")
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 42);
    assert_eq!(body["name"], "User 42");
}

#[tokio::test]
async fn test_secured_route_with_different_token() {
    let base = spawn_auth_app().await;
    let client = reqwest::Client::new();
    let resp = client.get(format!("{base}/api/me"))
        .header("Authorization", "Bearer valid-token-99")
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 99);
    assert_eq!(body["name"], "User 99");
}

#[tokio::test]
async fn test_unsecured_route_works_without_token() {
    let base = spawn_auth_app().await;
    let resp = reqwest::get(format!("{base}/api/health")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "healthy");
}

// ===== E2E Tests: OpenAPI Spec =====

#[tokio::test]
async fn test_openapi_secured_route_has_security() {
    let base = spawn_auth_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    // The secured route should have security requirement
    let get_me = &body["paths"]["/api/me"]["get"];
    let security = get_me["security"].as_array()
        .expect("Secured route should have security in OpenAPI spec");
    assert!(!security.is_empty(), "Security should not be empty");
    assert!(security.iter().any(|s| s.get("bearerAuth").is_some()),
        "Security should include bearerAuth");
}

#[tokio::test]
async fn test_openapi_secured_route_has_401_response() {
    let base = spawn_auth_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    let get_me = &body["paths"]["/api/me"]["get"];
    assert!(get_me["responses"]["401"].is_object(),
        "Secured route should have 401 response in OpenAPI");
    assert_eq!(get_me["responses"]["401"]["description"], "Unauthorized");
}

#[tokio::test]
async fn test_openapi_unsecured_route_has_no_security() {
    let base = spawn_auth_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    let health = &body["paths"]["/api/health"]["get"];
    // Should not have security requirement
    assert!(health.get("security").is_none() ||
            health["security"].as_array().map(|a| a.is_empty()).unwrap_or(true),
        "Unsecured route should not have security");
}

#[tokio::test]
async fn test_openapi_unsecured_route_has_no_401() {
    let base = spawn_auth_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    let health = &body["paths"]["/api/health"]["get"];
    assert!(!health["responses"]["401"].is_object(),
        "Unsecured route should not have 401 response");
}

#[tokio::test]
async fn test_openapi_has_bearer_auth_security_scheme() {
    let base = spawn_auth_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    let scheme = &body["components"]["securitySchemes"]["bearerAuth"];
    assert!(scheme.is_object(), "Should have bearerAuth security scheme");
    assert_eq!(scheme["type"], "http");
    assert_eq!(scheme["scheme"], "bearer");
}

// ===== Swagger UI =====

#[tokio::test]
async fn test_swagger_ui_available() {
    let base = spawn_auth_app().await;
    let resp = reqwest::get(format!("{base}/docs")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("swagger"), "Should serve Swagger UI HTML");
}
