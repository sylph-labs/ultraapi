// OAuth2 Dependency Tests
// Tests for OAuth2 dependency objects (OAuth2PasswordBearer, OAuth2AuthorizationCodeBearer)

use ultraapi::prelude::*;

// ============================================================================
// Basic OAuth2PasswordBearer Tests
// ============================================================================

#[get("/oauth2-password-protected")]
#[security("oauth2Password")]
async fn oauth2_password_protected(token: OAuth2PasswordBearer) -> String {
    format!("Token: {}", token.0)
}

#[tokio::test]
async fn test_oauth2_password_bearer_valid_token() {
    let app = UltraApiApp::new()
        .title("OAuth2 Password Bearer Test")
        .version("0.1.0")
        .oauth2_password(
            "oauth2Password",
            "https://example.com/token",
            [("read", "Read access"), ("write", "Write access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // With valid Bearer token
    let resp = client
        .get(format!("http://{}/oauth2-password-protected", addr))
        .header(
            "Authorization",
            "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test",
        )
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Token:"));
}

#[tokio::test]
async fn test_oauth2_password_bearer_missing_header() {
    let app = UltraApiApp::new()
        .title("OAuth2 Password Bearer Test")
        .version("0.1.0")
        .oauth2_password(
            "oauth2Password",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without Authorization header
    let resp = reqwest::get(format!("http://{}/oauth2-password-protected", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_oauth2_password_bearer_invalid_format() {
    let app = UltraApiApp::new()
        .title("OAuth2 Password Bearer Test")
        .version("0.1.0")
        .oauth2_password(
            "oauth2Password",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // With invalid format (Basic instead of Bearer)
    let resp = client
        .get(format!("http://{}/oauth2-password-protected", addr))
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ============================================================================
// OptionalOAuth2PasswordBearer Tests (auto_error=false)
// ============================================================================

#[get("/optional-oauth2-protected")]
#[security("oauth2Password")]
async fn optional_oauth2_protected(token: OptionalOAuth2PasswordBearer) -> String {
    match token.0 {
        Some(t) => format!("Token: {}", t),
        None => "No token provided".to_string(),
    }
}

#[tokio::test]
async fn test_optional_oauth2_password_bearer_valid_token() {
    let app = UltraApiApp::new()
        .title("Optional OAuth2 Password Bearer Test")
        .version("0.1.0")
        .oauth2_password(
            "oauth2Password",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // With valid Bearer token
    let resp = client
        .get(format!("http://{}/optional-oauth2-protected", addr))
        .header("Authorization", "Bearer valid-token-123")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Token: valid-token-123"));
}

#[tokio::test]
async fn test_optional_oauth2_password_bearer_missing_header() {
    let app = UltraApiApp::new()
        .title("Optional OAuth2 Password Bearer Test")
        .version("0.1.0")
        .oauth2_password(
            "oauth2Password",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without Authorization header - should return 200 with "No token"
    let resp = reqwest::get(format!("http://{}/optional-oauth2-protected", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("No token provided"));
}

#[tokio::test]
async fn test_optional_oauth2_password_bearer_invalid_format() {
    let app = UltraApiApp::new()
        .title("Optional OAuth2 Password Bearer Test")
        .version("0.1.0")
        .oauth2_password(
            "oauth2Password",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // With invalid format (Basic instead of Bearer) - should return 200 with None
    let resp = client
        .get(format!("http://{}/optional-oauth2-protected", addr))
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("No token provided"));
}

// ============================================================================
// OAuth2AuthorizationCodeBearer Tests
// ============================================================================

#[get("/oauth2-auth-code-protected")]
#[security("oauth2AuthCode")]
async fn oauth2_auth_code_protected(token: OAuth2AuthorizationCodeBearer) -> String {
    format!("Auth Code Token: {}", token.0)
}

#[tokio::test]
async fn test_oauth2_auth_code_bearer_valid_token() {
    let app = UltraApiApp::new()
        .title("OAuth2 Auth Code Bearer Test")
        .version("0.1.0")
        .oauth2_authorization_code(
            "oauth2AuthCode",
            "https://example.com/authorize",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // With valid Bearer token
    let resp = client
        .get(format!("http://{}/oauth2-auth-code-protected", addr))
        .header("Authorization", "Bearer auth-code-token-456")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Auth Code Token:"));
}

#[tokio::test]
async fn test_oauth2_auth_code_bearer_missing_header() {
    let app = UltraApiApp::new()
        .title("OAuth2 Auth Code Bearer Test")
        .version("0.1.0")
        .oauth2_authorization_code(
            "oauth2AuthCode",
            "https://example.com/authorize",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without Authorization header
    let resp = reqwest::get(format!("http://{}/oauth2-auth-code-protected", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ============================================================================
// OptionalOAuth2AuthorizationCodeBearer Tests
// ============================================================================

#[get("/optional-oauth2-auth-code-protected")]
#[security("oauth2AuthCode")]
async fn optional_oauth2_auth_code_protected(
    token: OptionalOAuth2AuthorizationCodeBearer,
) -> String {
    match token.0 {
        Some(t) => format!("Auth Code Token: {}", t),
        None => "No auth code token provided".to_string(),
    }
}

#[tokio::test]
async fn test_optional_oauth2_auth_code_bearer_missing_header() {
    let app = UltraApiApp::new()
        .title("Optional OAuth2 Auth Code Bearer Test")
        .version("0.1.0")
        .oauth2_authorization_code(
            "oauth2AuthCode",
            "https://example.com/authorize",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without Authorization header - should return 200
    let resp = reqwest::get(format!(
        "http://{}/optional-oauth2-auth-code-protected",
        addr
    ))
    .await
    .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("No auth code token provided"));
}

// ============================================================================
// OpenAPI Integration Tests
// ============================================================================

#[tokio::test]
async fn test_oauth2_password_in_openapi_spec() {
    let app = UltraApiApp::new()
        .title("OAuth2 Password OpenAPI Test")
        .version("0.1.0")
        .oauth2_password(
            "oauth2Password",
            "https://example.com/token",
            [("read", "Read access"), ("write", "Write access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let resp = reqwest::get(format!("http://{}/openapi.json", addr))
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    // Verify oauth2Password security scheme exists
    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("oauth2Password"));

    let oauth2 = &schemes["oauth2Password"];
    assert_eq!(oauth2["type"], "oauth2");
    assert!(oauth2["flows"].is_object());

    let flows = &oauth2["flows"];
    assert!(flows["password"].is_object());

    let password = &flows["password"];
    assert_eq!(password["tokenUrl"], "https://example.com/token");
    assert_eq!(password["scopes"]["read"], "Read access");
    assert_eq!(password["scopes"]["write"], "Write access");
}

#[tokio::test]
async fn test_oauth2_auth_code_in_openapi_spec() {
    let app = UltraApiApp::new()
        .title("OAuth2 Auth Code OpenAPI Test")
        .version("0.1.0")
        .oauth2_authorization_code(
            "oauth2AuthCode",
            "https://example.com/authorize",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let resp = reqwest::get(format!("http://{}/openapi.json", addr))
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    // Verify oauth2AuthCode security scheme exists
    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("oauth2AuthCode"));

    let oauth2 = &schemes["oauth2AuthCode"];
    assert_eq!(oauth2["type"], "oauth2");
    assert!(oauth2["flows"].is_object());

    let flows = &oauth2["flows"];
    assert!(flows["authorizationCode"].is_object());
}

// ============================================================================
// Error Response Format Tests
// ============================================================================

#[tokio::test]
async fn test_oauth2_error_response_format() {
    // Test that error response contains the expected format
    // The error should be in ApiError format
    let app = UltraApiApp::new()
        .title("OAuth2 Error Format Test")
        .version("0.1.0")
        .oauth2_password(
            "oauth2Password",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without Authorization header - should get error response
    let resp = reqwest::get(format!("http://{}/oauth2-password-protected", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);

    // Check error response format (should be JSON)
    let body: serde_json::Value = resp.json().await.unwrap();
    // ApiError typically has "error" or similar field
    assert!(body.get("error").is_some() || body.get("message").is_some());
}

// ============================================================================
// Utility Function Tests
// ============================================================================

#[test]
fn test_parse_bearer_token_valid() {
    use ultraapi::middleware::parse_bearer_token;

    let token = parse_bearer_token("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    assert!(token.is_some());
    assert_eq!(token.unwrap(), "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
}

#[test]
fn test_parse_bearer_token_lowercase() {
    use ultraapi::middleware::parse_bearer_token;

    // Should be case-insensitive
    let token = parse_bearer_token("bearer my-token-123");
    assert!(token.is_some());
    assert_eq!(token.unwrap(), "my-token-123");
}

#[test]
fn test_parse_bearer_token_invalid_scheme() {
    use ultraapi::middleware::parse_bearer_token;

    // Not Bearer scheme
    let token = parse_bearer_token("Basic dXNlcjpwYXNz");
    assert!(token.is_none());
}

#[test]
fn test_parse_bearer_token_empty_token() {
    use ultraapi::middleware::parse_bearer_token;

    // Empty token after Bearer
    let token = parse_bearer_token("Bearer ");
    assert!(token.is_none());
}

#[test]
fn test_parse_bearer_token_no_scheme() {
    use ultraapi::middleware::parse_bearer_token;

    // No Bearer scheme
    let token = parse_bearer_token("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    assert!(token.is_none());
}

#[test]
fn test_oauth2_scopes_new() {
    use ultraapi::middleware::OAuth2Scopes;

    let scopes = OAuth2Scopes::new(vec!["read".to_string(), "write".to_string()]);
    assert_eq!(scopes.scopes.len(), 2);
    assert!(scopes.scopes.contains(&"read".to_string()));
    assert!(scopes.scopes.contains(&"write".to_string()));
}

#[test]
fn test_oauth2_scopes_from_iter() {
    use ultraapi::middleware::OAuth2Scopes;

    let scopes = OAuth2Scopes::from_iter(["read", "write"]);
    assert_eq!(scopes.scopes.len(), 2);
}
