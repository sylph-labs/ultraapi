// P0 Security Tests
// Tests for bearer auth success/failure scenarios and OpenAPI security scheme emission

use ultraapi::prelude::*;

// ===== Protected Routes =====

#[get("/protected")]
#[security("bearer")]
async fn protected_route() -> String {
    "secret data".to_string()
}

#[get("/public")]
async fn public_route() -> String {
    "public data".to_string()
}

// ===== Helper =====

async fn spawn_app_with_security() -> String {
    let app = UltraApiApp::new()
        .title("Secure API")
        .version("0.1.0")
        .bearer_auth() // Register bearerAuth security scheme
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

// ===== Bearer Auth Tests =====

#[tokio::test]
async fn test_public_route_accessible_without_auth() {
    let base = spawn_app_with_security().await;
    let resp = reqwest::get(format!("{base}/public")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    // Response is JSON encoded
    assert!(body.contains("public data"));
}

#[tokio::test]
async fn test_protected_route_without_auth_currently_returns_200() {
    let base = spawn_app_with_security().await;
    let resp = reqwest::get(format!("{base}/protected")).await.unwrap();

    // UltraAPI currently documents security in OpenAPI but does not enforce it at runtime.
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_protected_route_with_valid_token_currently_returns_200() {
    let base = spawn_app_with_security().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/protected"))
        .header("Authorization", "Bearer valid-token")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("secret data"));
}

#[tokio::test]
async fn test_protected_route_with_invalid_token_currently_returns_200() {
    let base = spawn_app_with_security().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/protected"))
        .header("Authorization", "Bearer invalid-token")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_bearer_auth_scheme_in_openapi() {
    let base = spawn_app_with_security().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();

    // Verify bearerAuth security scheme is defined
    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(
        schemes.contains_key("bearerAuth"),
        "bearerAuth should be in securitySchemes"
    );

    let bearer = &schemes["bearerAuth"];
    assert_eq!(bearer["type"], "http");
    assert_eq!(bearer["scheme"], "bearer");
}

#[tokio::test]
async fn test_protected_route_has_security_requirement() {
    let base = spawn_app_with_security().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    // Get the protected route operation
    let protected = &body["paths"]["/protected"]["get"];
    let security = protected["security"].as_array().unwrap();

    assert!(
        !security.is_empty(),
        "Protected route should have security requirements"
    );
    assert!(
        security[0].get("bearerAuth").is_some(),
        "Protected route should require bearerAuth"
    );
}

#[tokio::test]
async fn test_public_route_no_security_requirement() {
    let base = spawn_app_with_security().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    // Get the public route operation
    let public = &body["paths"]["/public"]["get"];

    // Public route should either omit security or explicitly use an empty array.
    let security = public.get("security");
    assert!(
        security.is_none()
            || security
                .and_then(|s| s.as_array())
                .is_some_and(|s| s.is_empty()),
        "Public route should not require security"
    );
}

// ===== Security Scheme Emission Tests =====

#[test]
fn test_bearer_auth_scheme_creation() {
    // Verify bearer_auth() can be called and returns UltraApiApp
    let _app = UltraApiApp::new().title("Test API").bearer_auth();
}

#[test]
fn test_multiple_security_schemes() {
    // Test that multiple security schemes can be added
    use ultraapi::openapi::SecurityScheme;

    let api_key_scheme = SecurityScheme::ApiKey {
        name: "X-API-Key".to_string(),
        location: "header".to_string(),
    };

    let _app = UltraApiApp::new()
        .title("Multi-Security API")
        .security_scheme("apiKeyAuth", api_key_scheme)
        .bearer_auth();
}

// ===== Per-Route Security Tests =====

#[get("/admin-only")]
#[security("bearer")]
async fn admin_route() -> String {
    "admin area".to_string()
}

#[get("/user-or-admin")]
#[security("bearer")]
#[tag("user")]
async fn user_route() -> String {
    "user area".to_string()
}

#[get("/open")]
#[tag("public")]
async fn open_route() -> String {
    "open area".to_string()
}

#[tokio::test]
async fn test_per_route_security_in_openapi() {
    let app = UltraApiApp::new()
        .title("Per-Route Security Test")
        .version("0.1.0")
        .bearer_auth()
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

    // Admin route should require bearerAuth
    let admin_sec = &body["paths"]["/admin-only"]["get"]["security"];
    assert!(admin_sec
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.get("bearerAuth").is_some()));

    // User route should require bearerAuth
    let user_sec = &body["paths"]["/user-or-admin"]["get"]["security"];
    assert!(user_sec
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.get("bearerAuth").is_some()));

    // Open route should have no security requirement
    let open_op = &body["paths"]["/open"]["get"];
    let open_security = open_op.get("security");
    assert!(
        open_security.is_none()
            || open_security
                .and_then(|s| s.as_array())
                .is_some_and(|s| s.is_empty()),
        "Open route should not require security"
    );
}

// ===== Security Documentation Tests =====

#[tokio::test]
async fn test_security_scheme_documented_in_info() {
    let app = UltraApiApp::new()
        .title("Documented Security API")
        .version("1.0.0")
        .description("An API with bearer authentication")
        .bearer_auth()
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

    // Verify info is present
    assert_eq!(body["info"]["title"], "Documented Security API");
    assert_eq!(body["info"]["version"], "1.0.0");

    // Verify security schemes component exists
    assert!(body["components"].get("securitySchemes").is_some());
}

// ===== Auth Enforcement Note =====

// Note: UltraAPI currently documents security in OpenAPI but doesn't enforce it.
// Runtime auth enforcement should be implemented via custom middleware.
#[tokio::test]
async fn test_auth_enforcement_not_implemented_yet() {
    let base = spawn_app_with_security().await;

    // Without token
    let no_token = reqwest::get(format!("{base}/protected")).await.unwrap();

    // With valid token
    let client = reqwest::Client::new();
    let valid_token = client
        .get(format!("{base}/protected"))
        .header("Authorization", "Bearer valid-token")
        .send()
        .await
        .unwrap();

    // With invalid token
    let invalid_token = client
        .get(format!("{base}/protected"))
        .header("Authorization", "Bearer invalid-token")
        .send()
        .await
        .unwrap();

    // All requests currently succeed because auth is documentation-only.
    assert_eq!(no_token.status(), 200);
    assert_eq!(valid_token.status(), 200);
    assert_eq!(invalid_token.status(), 200);
}

// ===== API Key Security Scheme Test =====

#[get("/api-key-protected")]
#[security("apiKeyAuth")]
async fn api_key_protected_route() -> String {
    "api key protected".to_string()
}

#[tokio::test]
async fn test_api_key_security_scheme() {
    use ultraapi::openapi::SecurityScheme;

    let api_key_scheme = SecurityScheme::ApiKey {
        name: "X-API-Key".to_string(),
        location: "header".to_string(),
    };

    let app = UltraApiApp::new()
        .title("API Key Test")
        .version("0.1.0")
        .security_scheme("apiKeyAuth", api_key_scheme)
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

    // Verify apiKeyAuth security scheme is defined
    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("apiKeyAuth"));

    let api_key = &schemes["apiKeyAuth"];
    assert_eq!(api_key["type"], "apiKey");
    assert_eq!(api_key["name"], "X-API-Key");
    assert_eq!(api_key["in"], "header");
}

// ===== Router-Level Security Test =====

// Note: Router-level security requires routes to be registered via the router
// This test documents the capability
#[tokio::test]
async fn test_router_level_security_documented() {
    // Router-level security can be applied like:
    // ```ignore
    // let secure_router = UltraApiRouter::new("/api")
    //     .security("bearer") // Apply to all routes
    //     .route(my_handler);
    //
    // let app = UltraApiApp::new()
    //     .bearer_auth()
    //     .include(secure_router);
    // ```
    //
    // This test verifies the types are available
    let _router = UltraApiRouter::new("/test").security("bearer");
}

// ===== Auth Middleware Enforcement Tests =====

// These tests verify that the auth middleware actually enforces security requirements

use ultraapi::middleware::{AuthError, AuthValidator, Credentials};

async fn spawn_app_with_auth_enabled() -> String {
    let app = UltraApiApp::new()
        .title("Secure API")
        .version("0.1.0")
        .bearer_auth() // Register bearerAuth security scheme
        .middleware(|builder| {
            builder.enable_auth() // Enable auth middleware
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_auth_enforced_protected_route_without_auth_returns_401() {
    let base = spawn_app_with_auth_enabled().await;
    let resp = reqwest::get(format!("{base}/protected")).await.unwrap();

    // Should return 401 Unauthorized when no auth is provided
    assert_eq!(resp.status(), 401);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Missing authorization header"));
}

#[tokio::test]
async fn test_auth_enforced_protected_route_with_invalid_token_returns_401() {
    let base = spawn_app_with_auth_enabled().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/protected"))
        .header("Authorization", "Bearer invalid-token")
        .send()
        .await
        .unwrap();

    // Should return 401 Unauthorized for invalid token
    assert_eq!(resp.status(), 401);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Invalid or expired token"));
}

#[tokio::test]
async fn test_auth_enforced_protected_route_with_valid_token_returns_200() {
    let base = spawn_app_with_auth_enabled().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/protected"))
        .header("Authorization", "Bearer valid-token")
        .send()
        .await
        .unwrap();

    // Should return 200 OK for valid token
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("secret data"));
}

#[tokio::test]
async fn test_auth_enforced_protected_route_with_admin_token_returns_200() {
    let base = spawn_app_with_auth_enabled().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/protected"))
        .header("Authorization", "Bearer admin")
        .send()
        .await
        .unwrap();

    // Should return 200 OK for admin token
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("secret data"));
}

#[tokio::test]
async fn test_auth_enforced_public_route_without_auth_returns_200() {
    let app = UltraApiApp::new()
        .title("Secure API")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| builder.enable_auth())
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let resp = reqwest::get(format!("http://{}/public", addr))
        .await
        .unwrap();

    // Public routes should still be accessible without auth
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("public data"));
}

#[tokio::test]
async fn test_auth_with_api_key_protected_route() {
    // Test with custom API key validator
    #[get("/api-key-protected-test")]
    #[security("apiKeyAuth")]
    async fn api_key_route() -> String {
        "api key data".to_string()
    }

    let app = UltraApiApp::new()
        .title("API Key Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "X-API-Key".to_string(),
                location: "header".to_string(),
            },
        )
        .middleware(|builder| builder.enable_auth_with_api_keys(vec!["my-secret-key".to_string()]))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Without API key
    let resp = reqwest::get(format!("http://{}/api-key-protected-test", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With invalid API key
    let resp = client
        .get(format!("http://{}/api-key-protected-test", addr))
        .header("Authorization", "ApiKey wrong-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With valid API key
    let resp = client
        .get(format!("http://{}/api-key-protected-test", addr))
        .header("Authorization", "ApiKey my-secret-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_auth_middleware_with_custom_validator() {
    // Test with custom validator that only accepts "custom-token"
    #[get("/custom-protected-test")]
    #[security("bearer")]
    async fn custom_route() -> String {
        "custom protected".to_string()
    }

    #[derive(Clone)]
    struct CustomValidator;

    impl AuthValidator for CustomValidator {
        fn validate(&self, credentials: &Credentials) -> Result<(), AuthError> {
            if credentials.value == "custom-token" {
                Ok(())
            } else {
                Err(AuthError::unauthorized("Custom validation failed"))
            }
        }
    }

    let app = UltraApiApp::new()
        .title("Custom Validator Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| builder.enable_auth_with_validator(CustomValidator))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Without token
    let resp = reqwest::get(format!("http://{}/custom-protected-test", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With invalid token (valid- prefix but custom validator rejects)
    let resp = client
        .get(format!("http://{}/custom-protected-test", addr))
        .header("Authorization", "Bearer valid-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With custom token
    let resp = client
        .get(format!("http://{}/custom-protected-test", addr))
        .header("Authorization", "Bearer custom-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("custom protected"));
}

#[get("/protected-path/{id}")]
#[security("bearer")]
async fn protected_path_param_route(id: i64) -> String {
    format!("item-{id}")
}

#[tokio::test]
async fn test_auth_enforced_path_param_route_without_auth_returns_401() {
    let app = UltraApiApp::new()
        .title("Path Param Security Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| builder.enable_auth())
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let resp = reqwest::get(format!("http://{}/protected-path/42", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[get("/method-mix")]
async fn method_mix_public() -> String {
    "public get".to_string()
}

#[post("/method-mix")]
#[security("bearer")]
async fn method_mix_protected() -> String {
    "secure post".to_string()
}

#[tokio::test]
async fn test_auth_enforcement_respects_http_method() {
    let app = UltraApiApp::new()
        .title("Method Security Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| builder.enable_auth())
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    let get_resp = client
        .get(format!("http://{}/method-mix", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(get_resp.status(), 200);

    let post_resp = client
        .post(format!("http://{}/method-mix", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(post_resp.status(), 401);
}

#[get("/router-level-auth")]
async fn router_level_auth_target() -> String {
    "router secure".to_string()
}

#[tokio::test]
async fn test_router_level_security_enforced_at_runtime() {
    let secured = UltraApiRouter::new("/secure")
        .security("bearer")
        .route(__HAYAI_ROUTE_ROUTER_LEVEL_AUTH_TARGET);

    let app = UltraApiApp::new()
        .title("Router-Level Security Runtime Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| builder.enable_auth())
        .include(secured)
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    let unauth = client
        .get(format!("http://{}/secure/router-level-auth", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(unauth.status(), 401);

    let authed = client
        .get(format!("http://{}/secure/router-level-auth", addr))
        .header("Authorization", "Bearer valid-token")
        .send()
        .await
        .unwrap();
    assert_eq!(authed.status(), 200);
}

#[get("/forbidden-policy")]
#[security("bearer")]
async fn forbidden_policy_route() -> String {
    "blocked".to_string()
}

#[tokio::test]
async fn test_auth_validator_forbidden_maps_to_403() {
    #[derive(Clone)]
    struct ForbiddenValidator;

    impl AuthValidator for ForbiddenValidator {
        fn validate(&self, _credentials: &Credentials) -> Result<(), AuthError> {
            Err(AuthError::forbidden("Forbidden by policy"))
        }
    }

    let app = UltraApiApp::new()
        .title("Forbidden Mapping Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| builder.enable_auth_with_validator(ForbiddenValidator))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let resp = reqwest::Client::new()
        .get(format!("http://{}/forbidden-policy", addr))
        .header("Authorization", "Bearer anything")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Forbidden by policy"));
}

#[test]
fn test_cors_default_build_does_not_panic_with_wildcard_header() {
    let _ = ultraapi::middleware::CorsConfig::new().build();
}

// =============================================================================
// API Key in Query Parameter Tests
// =============================================================================

#[get("/query-protected")]
#[security("apiKeyAuth")]
async fn query_protected_route() -> String {
    "query protected data".to_string()
}

#[tokio::test]
async fn test_api_key_in_query_success() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("API Key Query Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "api_key".to_string(),
                location: "query".to_string(),
            },
        )
        .middleware(|builder| {
            builder
                .enable_auth_with_api_keys(vec!["valid-key".to_string()])
                .with_security_scheme(SecuritySchemeConfig::api_key_query("apiKeyAuth", "api_key"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // With valid API key in query
    let resp = reqwest::get(format!("http://{}/query-protected?api_key=valid-key", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("query protected data"));
}

#[tokio::test]
async fn test_api_key_in_query_failure() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("API Key Query Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "api_key".to_string(),
                location: "query".to_string(),
            },
        )
        .middleware(|builder| {
            builder
                .enable_auth_with_api_keys(vec!["valid-key".to_string()])
                .with_security_scheme(SecuritySchemeConfig::api_key_query("apiKeyAuth", "api_key"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without API key
    let resp = reqwest::get(format!("http://{}/query-protected", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With invalid API key
    let resp = reqwest::get(format!("http://{}/query-protected?api_key=wrong-key", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

// =============================================================================
// API Key in Cookie Tests
// =============================================================================

#[get("/cookie-protected")]
#[security("apiKeyAuth")]
async fn cookie_protected_route() -> String {
    "cookie protected data".to_string()
}

#[tokio::test]
async fn test_api_key_in_cookie_success() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("API Key Cookie Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "api_key".to_string(),
                location: "cookie".to_string(),
            },
        )
        .middleware(|builder| {
            builder
                .enable_auth_with_api_keys(vec!["valid-key".to_string()])
                .with_security_scheme(SecuritySchemeConfig::api_key_cookie(
                    "apiKeyAuth",
                    "api_key",
                ))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // With valid API key in cookie
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/cookie-protected", addr))
        .header("Cookie", "api_key=valid-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("cookie protected data"));
}

#[tokio::test]
async fn test_api_key_in_cookie_failure() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("API Key Cookie Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "api_key".to_string(),
                location: "cookie".to_string(),
            },
        )
        .middleware(|builder| {
            builder
                .enable_auth_with_api_keys(vec!["valid-key".to_string()])
                .with_security_scheme(SecuritySchemeConfig::api_key_cookie(
                    "apiKeyAuth",
                    "api_key",
                ))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without cookie
    let resp = reqwest::get(format!("http://{}/cookie-protected", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With invalid API key in cookie
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/cookie-protected", addr))
        .header("Cookie", "api_key=wrong-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

// =============================================================================
// API Key in Header Tests (explicit)
// =============================================================================

#[get("/header-api-key-protected")]
#[security("apiKeyAuth")]
async fn header_api_key_route() -> String {
    "header api key data".to_string()
}

#[tokio::test]
async fn test_api_key_in_custom_header_success() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("API Key Header Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "X-API-Key".to_string(),
                location: "header".to_string(),
            },
        )
        .middleware(|builder| {
            builder
                .enable_auth_with_api_keys(vec!["my-secret-key".to_string()])
                .with_security_scheme(SecuritySchemeConfig::api_key_header(
                    "apiKeyAuth",
                    "X-API-Key",
                ))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // With valid API key in custom header
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/header-api-key-protected", addr))
        .header("X-API-Key", "my-secret-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("header api key data"));
}

#[tokio::test]
async fn test_api_key_in_custom_header_failure() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("API Key Header Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "X-API-Key".to_string(),
                location: "header".to_string(),
            },
        )
        .middleware(|builder| {
            builder
                .enable_auth_with_api_keys(vec!["my-secret-key".to_string()])
                .with_security_scheme(SecuritySchemeConfig::api_key_header(
                    "apiKeyAuth",
                    "X-API-Key",
                ))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without header
    let resp = reqwest::get(format!("http://{}/header-api-key-protected", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With wrong API key
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/header-api-key-protected", addr))
        .header("X-API-Key", "wrong-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

// =============================================================================
// Scope Validation Tests
// =============================================================================

#[get("/scope-read")]
#[security("bearer")]
async fn scope_read_route() -> String {
    "read data".to_string()
}

#[tokio::test]
async fn test_scope_validation_success() {
    use ultraapi::middleware::{MockAuthValidator, ScopedAuthValidator, SecuritySchemeConfig};

    let validator = ScopedAuthValidator::new(MockAuthValidator::new())
        .with_scope("valid-read", vec!["read".to_string()])
        .with_scope("valid-write", vec!["read".to_string(), "write".to_string()])
        .with_scope(
            "admin-token",
            vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        );

    let app = UltraApiApp::new()
        .title("Scope Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_validator(validator)
                .with_security_scheme(
                    SecuritySchemeConfig::bearer("bearerAuth")
                        .with_scopes(vec!["read".to_string()]),
                )
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // With valid-read token (has read scope)
    let resp = client
        .get(format!("http://{}/scope-read", addr))
        .header("Authorization", "Bearer valid-read")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("read data"));
}

#[tokio::test]
async fn test_scope_validation_failure() {
    use ultraapi::middleware::{MockAuthValidator, ScopedAuthValidator, SecuritySchemeConfig};

    // Validator that only grants 'write' scope to 'valid-write' token
    let validator = ScopedAuthValidator::new(MockAuthValidator::new())
        .with_scope("valid-write", vec!["write".to_string()]);

    let app = UltraApiApp::new()
        .title("Scope Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_validator(validator)
                .with_security_scheme(
                    SecuritySchemeConfig::bearer("bearerAuth")
                        .with_scopes(vec!["read".to_string()]), // Requires read scope
                )
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // With valid-write token (only has write scope, but route requires read)
    let resp = client
        .get(format!("http://{}/scope-read", addr))
        .header("Authorization", "Bearer valid-write")
        .send()
        .await
        .unwrap();
    // Should fail with 403 Forbidden due to insufficient scope
    assert_eq!(resp.status(), 403);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Insufficient scope"));
}

// =============================================================================
// Public Routes Remain Unaffected Tests
// =============================================================================

#[get("/public-no-auth")]
async fn public_no_auth_route() -> String {
    "public always".to_string()
}

#[tokio::test]
async fn test_public_routes_unaffected_by_auth_middleware() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("Public Routes Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| {
            builder
                .enable_auth()
                .with_security_scheme(SecuritySchemeConfig::bearer("bearerAuth"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Public route should be accessible without auth
    let resp = reqwest::get(format!("http://{}/public-no-auth", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("public always"));
}

#[tokio::test]
async fn test_protected_routes_require_auth() {
    use ultraapi::middleware::SecuritySchemeConfig;

    #[get("/should-be-protected")]
    #[security("bearer")]
    async fn should_be_protected() -> String {
        "protected".to_string()
    }

    let app = UltraApiApp::new()
        .title("Protected Routes Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| {
            builder
                .enable_auth()
                .with_security_scheme(SecuritySchemeConfig::bearer("bearerAuth"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without auth - should fail
    let resp = reqwest::get(format!("http://{}/should-be-protected", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With valid token - should succeed
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/should-be-protected", addr))
        .header("Authorization", "Bearer valid-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

// =============================================================================
// Backward Compatibility Tests
// =============================================================================

#[tokio::test]
async fn test_bearer_auth_backward_compatibility() {
    // Test that existing bearer auth still works without explicit security scheme config
    let app = UltraApiApp::new()
        .title("Backward Compatibility Test")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| builder.enable_auth())
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Without auth
    let resp = reqwest::get(format!("http://{}/protected", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With valid token
    let resp = client
        .get(format!("http://{}/protected", addr))
        .header("Authorization", "Bearer valid-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // With invalid token
    let resp = client
        .get(format!("http://{}/protected", addr))
        .header("Authorization", "Bearer invalid-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

// =============================================================================
// OpenAPI Security Scheme Alignment Tests
// =============================================================================

#[tokio::test]
async fn test_openapi_query_api_key_scheme() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("OpenAPI Query API Key Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "api_key".to_string(),
                location: "query".to_string(),
            },
        )
        .middleware(|builder| {
            builder
                .enable_auth_with_api_keys(vec!["test-key".to_string()])
                .with_security_scheme(SecuritySchemeConfig::api_key_query("apiKeyAuth", "api_key"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Check OpenAPI spec
    let resp = reqwest::get(format!("http://{}/openapi.json", addr))
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    let scheme = &body["components"]["securitySchemes"]["apiKeyAuth"];
    assert_eq!(scheme["type"], "apiKey");
    assert_eq!(scheme["name"], "api_key");
    assert_eq!(scheme["in"], "query");

    // Check the route has security requirement
    let route = &body["paths"]["/query-protected"]["get"];
    let security = route["security"].as_array().unwrap();
    assert!(security.iter().any(|s| s.get("apiKeyAuth").is_some()));
}

#[tokio::test]
async fn test_openapi_cookie_api_key_scheme() {
    use ultraapi::middleware::SecuritySchemeConfig;

    let app = UltraApiApp::new()
        .title("OpenAPI Cookie API Key Test")
        .version("0.1.0")
        .security_scheme(
            "apiKeyAuth",
            ultraapi::openapi::SecurityScheme::ApiKey {
                name: "session".to_string(),
                location: "cookie".to_string(),
            },
        )
        .middleware(|builder| {
            builder
                .enable_auth_with_api_keys(vec!["test-session".to_string()])
                .with_security_scheme(SecuritySchemeConfig::api_key_cookie(
                    "apiKeyAuth",
                    "session",
                ))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Check OpenAPI spec
    let resp = reqwest::get(format!("http://{}/openapi.json", addr))
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    let scheme = &body["components"]["securitySchemes"]["apiKeyAuth"];
    assert_eq!(scheme["type"], "apiKey");
    assert_eq!(scheme["name"], "session");
    assert_eq!(scheme["in"], "cookie");
}
