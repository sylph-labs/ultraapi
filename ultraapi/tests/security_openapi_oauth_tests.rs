// P0 Security OAuth2/OpenID Connect Tests
// Tests for OAuth2 and OpenID Connect security scheme serialization in OpenAPI

use ultraapi::prelude::*;

// ===== OAuth2 Implicit Flow Tests =====

#[get("/oauth2-implicit-protected")]
#[security("oauth2Implicit")]
async fn oauth2_implicit_protected() -> String {
    "oauth2 implicit data".to_string()
}

#[tokio::test]
async fn test_oauth2_implicit_flow_scheme_serialization() {
    let app = UltraApiApp::new()
        .title("OAuth2 Implicit Test")
        .version("0.1.0")
        .oauth2_implicit(
            "oauth2Implicit",
            "https://example.com/authorize",
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

    // Verify oauth2Implicit security scheme exists
    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("oauth2Implicit"));

    let oauth2 = &schemes["oauth2Implicit"];
    assert_eq!(oauth2["type"], "oauth2");
    assert!(oauth2["flows"].is_object());

    let flows = &oauth2["flows"];
    assert!(flows["implicit"].is_object());

    let implicit = &flows["implicit"];
    assert_eq!(
        implicit["authorizationUrl"],
        "https://example.com/authorize"
    );
    assert!(implicit["scopes"].is_object());
    assert_eq!(implicit["scopes"]["read"], "Read access");
    assert_eq!(implicit["scopes"]["write"], "Write access");
}

#[tokio::test]
async fn test_oauth2_implicit_route_has_security_with_scopes() {
    // Note: Current implementation requires explicit scope specification in security attribute
    // This test verifies that security requirement is present
    let app = UltraApiApp::new()
        .title("OAuth2 Implicit Scope Test")
        .version("0.1.0")
        .oauth2_implicit(
            "oauth2Implicit",
            "https://example.com/authorize",
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

    // Verify route has security requirement
    let route_security = &body["paths"]["/oauth2-implicit-protected"]["get"]["security"];
    let security_arr = route_security.as_array().unwrap();
    assert!(!security_arr.is_empty());
    assert!(security_arr[0].get("oauth2Implicit").is_some());
}

// ===== OAuth2 Password Flow Tests =====

#[get("/oauth2-password-protected")]
#[security("oauth2Password")]
async fn oauth2_password_protected() -> String {
    "oauth2 password data".to_string()
}

#[tokio::test]
async fn test_oauth2_password_flow_scheme_serialization() {
    let app = UltraApiApp::new()
        .title("OAuth2 Password Test")
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("oauth2Password"));

    let oauth2 = &schemes["oauth2Password"];
    assert_eq!(oauth2["type"], "oauth2");

    let flows = &oauth2["flows"];
    let password = &flows["password"];
    assert_eq!(password["tokenUrl"], "https://example.com/token");
    assert_eq!(password["scopes"]["read"], "Read access");
    assert_eq!(password["scopes"]["write"], "Write access");
}

// ===== OAuth2 Client Credentials Flow Tests =====

#[get("/oauth2-client-creds-protected")]
#[security("oauth2ClientCredentials")]
async fn oauth2_client_creds_protected() -> String {
    "oauth2 client creds data".to_string()
}

#[tokio::test]
async fn test_oauth2_client_credentials_flow_serialization() {
    let app = UltraApiApp::new()
        .title("OAuth2 Client Credentials Test")
        .version("0.1.0")
        .oauth2_client_credentials(
            "oauth2ClientCredentials",
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("oauth2ClientCredentials"));

    let oauth2 = &schemes["oauth2ClientCredentials"];
    assert_eq!(oauth2["type"], "oauth2");

    let flows = &oauth2["flows"];
    let client_creds = &flows["clientCredentials"];
    assert_eq!(client_creds["tokenUrl"], "https://example.com/token");
    assert_eq!(client_creds["scopes"]["read"], "Read access");
}

// ===== OAuth2 Authorization Code Flow Tests =====

#[get("/oauth2-auth-code-protected")]
#[security("oauth2AuthCode")]
async fn oauth2_auth_code_protected() -> String {
    "oauth2 auth code data".to_string()
}

#[tokio::test]
async fn test_oauth2_authorization_code_flow_serialization() {
    let app = UltraApiApp::new()
        .title("OAuth2 Auth Code Test")
        .version("0.1.0")
        .oauth2_authorization_code(
            "oauth2AuthCode",
            "https://example.com/authorize",
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("oauth2AuthCode"));

    let oauth2 = &schemes["oauth2AuthCode"];
    assert_eq!(oauth2["type"], "oauth2");

    let flows = &oauth2["flows"];
    let auth_code = &flows["authorizationCode"];
    assert_eq!(
        auth_code["authorizationUrl"],
        "https://example.com/authorize"
    );
    assert_eq!(auth_code["tokenUrl"], "https://example.com/token");
    assert_eq!(auth_code["scopes"]["read"], "Read access");
}

// ===== OAuth2 Multiple Flows Tests =====

#[tokio::test]
async fn test_oauth2_multiple_flows_serialization() {
    use ultraapi::openapi::{OAuth2Flow, OAuth2Flows};

    let flows = OAuth2Flows {
        authorization_code: Some(OAuth2Flow {
            authorization_url: Some("https://example.com/authorize".to_string()),
            token_url: Some("https://example.com/token".to_string()),
            refresh_url: Some("https://example.com/refresh".to_string()),
            scopes: [
                ("read".to_string(), "Read access".to_string()),
                ("write".to_string(), "Write access".to_string()),
            ]
            .into(),
        }),
        client_credentials: Some(OAuth2Flow {
            authorization_url: None,
            token_url: Some("https://example.com/token".to_string()),
            refresh_url: None,
            scopes: [("server".to_string(), "Server access".to_string())].into(),
        }),
        ..Default::default()
    };

    let app = UltraApiApp::new()
        .title("OAuth2 Multi Flow Test")
        .version("0.1.0")
        .oauth2("oauth2Multi", flows)
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    let oauth2 = &schemes["oauth2Multi"];

    let flows_obj = &oauth2["flows"];
    // Both flows should be present
    assert!(flows_obj["authorizationCode"].is_object());
    assert!(flows_obj["clientCredentials"].is_object());
}

// ===== OpenID Connect Tests =====

#[get("/oidc-protected")]
#[security("oidc")]
async fn oidc_protected() -> String {
    "oidc data".to_string()
}

#[tokio::test]
async fn test_openid_connect_scheme_serialization() {
    let app = UltraApiApp::new()
        .title("OpenID Connect Test")
        .version("0.1.0")
        .openid_connect(
            "oidc",
            "https://example.com/.well-known/openid-configuration",
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("oidc"));

    let oidc = &schemes["oidc"];
    assert_eq!(oidc["type"], "openIdConnect");
    assert_eq!(
        oidc["openIdConnectUrl"],
        "https://example.com/.well-known/openid-configuration"
    );
}

#[tokio::test]
async fn test_openid_connect_route_has_security_requirement() {
    let app = UltraApiApp::new()
        .title("OpenID Connect Route Test")
        .version("0.1.0")
        .openid_connect(
            "oidc",
            "https://example.com/.well-known/openid-configuration",
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

    let route_security = &body["paths"]["/oidc-protected"]["get"]["security"];
    let security_arr = route_security.as_array().unwrap();
    assert!(!security_arr.is_empty());
    assert!(security_arr[0].get("oidc").is_some());
}

// ===== API Key Helper Method Tests =====

#[get("/api-key-helper-protected")]
#[security("apiKeyHelper")]
async fn api_key_helper_protected() -> String {
    "api key helper data".to_string()
}

#[tokio::test]
async fn test_api_key_helper_method_serialization() {
    let app = UltraApiApp::new()
        .title("API Key Helper Test")
        .version("0.1.0")
        .api_key("apiKeyHelper", "X-API-Key", "header")
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("apiKeyHelper"));

    let api_key = &schemes["apiKeyHelper"];
    assert_eq!(api_key["type"], "apiKey");
    assert_eq!(api_key["name"], "X-API-Key");
    assert_eq!(api_key["in"], "header");
}

// ===== Combined Security Schemes Tests =====

#[get("/combined-bearer-oauth2")]
#[security("bearerAuth")]
async fn combined_bearer_route() -> String {
    "bearer data".to_string()
}

#[get("/combined-oauth2")]
#[security("oauth2AuthCode")]
async fn combined_oauth2_route() -> String {
    "oauth2 data".to_string()
}

#[get("/combined-public")]
async fn combined_public_route() -> String {
    "public data".to_string()
}

#[tokio::test]
async fn test_combined_security_schemes() {
    let app = UltraApiApp::new()
        .title("Combined Security Test")
        .version("0.1.0")
        .bearer_auth()
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();

    // Both bearerAuth and oauth2AuthCode should exist
    assert!(schemes.contains_key("bearerAuth"));
    assert!(schemes.contains_key("oauth2AuthCode"));

    // Verify bearerAuth structure
    let bearer = &schemes["bearerAuth"];
    assert_eq!(bearer["type"], "http");
    assert_eq!(bearer["scheme"], "bearer");

    // Verify oauth2AuthCode structure
    let oauth2 = &schemes["oauth2AuthCode"];
    assert_eq!(oauth2["type"], "oauth2");
}

#[tokio::test]
async fn test_route_level_security_with_multiple_schemes() {
    let app = UltraApiApp::new()
        .title("Route Security Combo Test")
        .version("0.1.0")
        .bearer_auth()
        .oauth2_authorization_code(
            "oauth2AuthCode",
            "https://example.com/authorize",
            "https://example.com/token",
            [("read", "Read access")],
        )
        .openid_connect(
            "oidc",
            "https://example.com/.well-known/openid-configuration",
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

    // Check bearer route requires bearerAuth
    let bearer_sec = &body["paths"]["/combined-bearer-oauth2"]["get"]["security"];
    assert!(bearer_sec
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.get("bearerAuth").is_some()));

    // Check oauth2 route requires oauth2AuthCode
    let oauth2_sec = &body["paths"]["/combined-oauth2"]["get"]["security"];
    assert!(oauth2_sec
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s.get("oauth2AuthCode").is_some()));

    // Check public route has no security requirement
    let public_sec = &body["paths"]["/combined-public"]["get"]["security"];
    assert!(public_sec.is_null() || public_sec.as_array().is_some_and(|s| s.is_empty()));
}

// ===== App-Level + Route-Level Combined Tests =====

#[get("/app-level-oauth2")]
#[security("oauth2AppLevel")]
async fn app_level_oauth2_route() -> String {
    "app level oauth2".to_string()
}

#[tokio::test]
async fn test_app_level_oauth2_with_route_level_bearer() {
    // App-level OAuth2 + route-level bearer
    let app = UltraApiApp::new()
        .title("App+Route Security Test")
        .version("0.1.0")
        .oauth2_authorization_code(
            "oauth2AppLevel",
            "https://example.com/authorize",
            "https://example.com/token",
            [("read", "Read access")],
        )
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();

    // Both should exist
    assert!(schemes.contains_key("oauth2AppLevel"));
    assert!(schemes.contains_key("bearerAuth"));
}

// ===== Security Scheme with Scopes in Route =====
// Note: Currently UltraAPI doesn't support parsing scopes from security attributes like
// #[security("oauth2AuthCode:read")]. This is a known limitation.
// The security requirement is created but scopes are empty arrays.

#[get("/scoped-oauth2")]
#[security("oauth2AuthCode")]
async fn scoped_oauth2() -> String {
    "scoped access".to_string()
}

#[tokio::test]
async fn test_route_security_with_specific_scopes() {
    // Note: Current implementation doesn't parse scopes from security attribute syntax
    // This test verifies basic security requirement is present
    let app = UltraApiApp::new()
        .title("Scoped Security Test")
        .version("0.1.0")
        .oauth2_authorization_code(
            "oauth2AuthCode",
            "https://example.com/authorize",
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

    // Verify route has security requirement
    let route_security = &body["paths"]["/scoped-oauth2"]["get"]["security"];
    let security_arr = route_security.as_array().unwrap();
    assert!(!security_arr.is_empty());
    assert!(security_arr[0].get("oauth2AuthCode").is_some());
}

// ===== Raw SecurityScheme Test =====

#[tokio::test]
async fn test_raw_security_scheme_builder() {
    use ultraapi::openapi::{OAuth2Flow, OAuth2Flows, SecurityScheme};

    let flows = OAuth2Flows {
        password: Some(OAuth2Flow {
            authorization_url: None,
            token_url: Some("https://example.com/token".to_string()),
            refresh_url: Some("https://example.com/refresh".to_string()),
            scopes: [("admin".to_string(), "Admin access".to_string())].into(),
        }),
        ..Default::default()
    };

    let app = UltraApiApp::new()
        .title("Raw Security Scheme Test")
        .version("0.1.0")
        .security_scheme("rawOAuth2", SecurityScheme::OAuth2(flows))
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

    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("rawOAuth2"));

    let raw = &schemes["rawOAuth2"];
    assert_eq!(raw["type"], "oauth2");
    assert!(raw["flows"]["password"].is_object());
}
