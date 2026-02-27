// Basic Authentication Tests
// Tests for HTTP Basic authentication support

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ultraapi::prelude::*;
use ultraapi::middleware::{
    decode_basic_header, parse_basic_header, BasicCredentials, BasicAuthValidator,
    SecuritySchemeConfig, Credentials, AuthValidator, CredentialLocation,
};

// ============================================================================
// Basic Auth Decode Utility Tests
// ============================================================================

#[test]
fn test_decode_basic_header_valid() {
    // "admin:secret123" encoded in base64
    let encoded = BASE64.encode(b"admin:secret123");
    let result = decode_basic_header(&encoded);
    
    assert!(result.is_some());
    let creds = result.unwrap();
    assert_eq!(creds.username, "admin");
    assert_eq!(creds.password, "secret123");
}

#[test]
fn test_decode_basic_header_with_colon_in_password() {
    // "admin:p@ss:word" - password contains colon
    let encoded = BASE64.encode(b"admin:p@ss:word");
    let result = decode_basic_header(&encoded);
    
    assert!(result.is_some());
    let creds = result.unwrap();
    assert_eq!(creds.username, "admin");
    assert_eq!(creds.password, "p@ss:word");
}

#[test]
fn test_decode_basic_header_invalid_base64() {
    // Invalid base64 string
    let result = decode_basic_header("not-valid-base64!!!");
    assert!(result.is_none());
}

#[test]
fn test_decode_basic_header_no_colon() {
    // String without colon separator
    let encoded = BASE64.encode(b"nocolon");
    let result = decode_basic_header(&encoded);
    assert!(result.is_none());
}

#[test]
fn test_decode_basic_header_empty() {
    let result = decode_basic_header("");
    assert!(result.is_none());
}

#[test]
fn test_parse_basic_header_with_prefix() {
    let result = parse_basic_header("Basic YWRtaW46c2VjcmV0MTIz");
    assert!(result.is_some());
    let creds = result.unwrap();
    assert_eq!(creds.username, "admin");
    assert_eq!(creds.password, "secret123");
}

#[test]
fn test_parse_basic_header_lowercase_prefix() {
    let result = parse_basic_header("basic YWRtaW46c2VjcmV0MTIz");
    assert!(result.is_some());
    let creds = result.unwrap();
    assert_eq!(creds.username, "admin");
    assert_eq!(creds.password, "secret123");
}

#[test]
fn test_parse_basic_header_no_prefix() {
    let result = parse_basic_header("YWRtaW46c2VjcmV0MTIz");
    assert!(result.is_none());
}

// ============================================================================
// Credentials Basic Auth Tests
// ============================================================================

#[test]
fn test_credentials_from_basic() {
    let basic = BasicCredentials {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    let creds = Credentials::from_basic(basic, "basicAuth");
    
    assert!(creds.is_basic());
    assert_eq!(creds.scheme, "basic");
    assert_eq!(creds.username.as_deref(), Some("user"));
    assert_eq!(creds.password.as_deref(), Some("pass"));
    assert_eq!(creds.security_scheme.as_deref(), Some("basicAuth"));
}

#[test]
fn test_credentials_is_basic() {
    let basic_creds = Credentials::new("basic", "encoded");
    assert!(basic_creds.is_basic());
    
    let bearer_creds = Credentials::new("bearer", "token");
    assert!(!bearer_creds.is_basic());
    
    let apikey_creds = Credentials::new("ApiKey", "key");
    assert!(!apikey_creds.is_basic());
}

#[test]
fn test_credentials_basic_username_password() {
    let basic = BasicCredentials {
        username: "admin".to_string(),
        password: "password123".to_string(),
    };
    let creds = Credentials::from_basic(basic, "basicAuth");
    
    assert_eq!(creds.basic_username(), Some("admin"));
    assert_eq!(creds.basic_password(), Some("password123"));
}

// ============================================================================
// BasicAuthValidator Tests
// ============================================================================

#[test]
fn test_basic_auth_validator_valid_credentials() {
    let validator = BasicAuthValidator::new(vec![
        ("admin".to_string(), "secret123".to_string()),
        ("user".to_string(), "password".to_string()),
    ]);
    
    let basic = BasicCredentials {
        username: "admin".to_string(),
        password: "secret123".to_string(),
    };
    let creds = Credentials::from_basic(basic, "basicAuth");
    
    assert!(validator.validate(&creds).is_ok());
}

#[test]
fn test_basic_auth_validator_invalid_password() {
    let validator = BasicAuthValidator::new(vec![
        ("admin".to_string(), "secret123".to_string()),
    ]);
    
    let basic = BasicCredentials {
        username: "admin".to_string(),
        password: "wrongpassword".to_string(),
    };
    let creds = Credentials::from_basic(basic, "basicAuth");
    
    assert!(validator.validate(&creds).is_err());
}

#[test]
fn test_basic_auth_validator_invalid_username() {
    let validator = BasicAuthValidator::new(vec![
        ("admin".to_string(), "secret123".to_string()),
    ]);
    
    let basic = BasicCredentials {
        username: "wronguser".to_string(),
        password: "secret123".to_string(),
    };
    let creds = Credentials::from_basic(basic, "basicAuth");
    
    assert!(validator.validate(&creds).is_err());
}

#[test]
fn test_basic_auth_validator_non_basic_scheme() {
    let validator = BasicAuthValidator::new(vec![
        ("admin".to_string(), "secret123".to_string()),
    ]);
    
    let creds = Credentials::new("bearer", "some-token");
    
    assert!(validator.validate(&creds).is_err());
}

#[test]
fn test_basic_auth_validator_with_credential_method() {
    let validator = BasicAuthValidator::new(vec![])
        .with_credential("admin", "admin123")
        .with_credential("user", "user123");
    
    let basic = BasicCredentials {
        username: "admin".to_string(),
        password: "admin123".to_string(),
    };
    let creds = Credentials::from_basic(basic, "basicAuth");
    
    assert!(validator.validate(&creds).is_ok());
}

// ============================================================================
// SecuritySchemeConfig Basic Tests
// ============================================================================

#[test]
fn test_security_scheme_config_basic() {
    let config = SecuritySchemeConfig::basic("basicAuth");
    
    assert_eq!(config.name, "basicAuth");
    assert_eq!(config.location, CredentialLocation::Header);
    assert_eq!(config.param_name, "authorization");
    assert!(config.scopes.is_empty());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[get("/basic-protected")]
#[security("basicAuth")]
async fn basic_protected_route() -> String {
    "basic protected data".to_string()
}

#[tokio::test]
async fn test_basic_auth_success() {
    use ultraapi::middleware::SecuritySchemeConfig;
    
    let app = UltraApiApp::new()
        .title("Basic Auth Test")
        .version("0.1.0")
        .basic_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_basic(vec![
                    ("admin".to_string(), "secret123".to_string()),
                ])
                .with_security_scheme(SecuritySchemeConfig::basic("basicAuth"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Encode credentials
    let encoded = BASE64.encode(b"admin:secret123");
    
    // With valid credentials
    let resp = client
        .get(format!("http://{}/basic-protected", addr))
        .header("Authorization", format!("Basic {}", encoded))
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("basic protected data"));
}

#[tokio::test]
async fn test_basic_auth_failure_no_credentials() {
    use ultraapi::middleware::SecuritySchemeConfig;
    
    let app = UltraApiApp::new()
        .title("Basic Auth Test")
        .version("0.1.0")
        .basic_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_basic(vec![
                    ("admin".to_string(), "secret123".to_string()),
                ])
                .with_security_scheme(SecuritySchemeConfig::basic("basicAuth"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without credentials
    let resp = reqwest::get(format!("http://{}/basic-protected", addr))
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_basic_auth_failure_invalid_credentials() {
    use ultraapi::middleware::SecuritySchemeConfig;
    
    let app = UltraApiApp::new()
        .title("Basic Auth Test")
        .version("0.1.0")
        .basic_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_basic(vec![
                    ("admin".to_string(), "secret123".to_string()),
                ])
                .with_security_scheme(SecuritySchemeConfig::basic("basicAuth"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Encode wrong credentials
    let encoded = BASE64.encode(b"admin:wrongpassword");
    
    // With invalid credentials
    let resp = client
        .get(format!("http://{}/basic-protected", addr))
        .header("Authorization", format!("Basic {}", encoded))
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_basic_auth_invalid_base64() {
    use ultraapi::middleware::SecuritySchemeConfig;
    
    let app = UltraApiApp::new()
        .title("Basic Auth Test")
        .version("0.1.0")
        .basic_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_basic(vec![
                    ("admin".to_string(), "secret123".to_string()),
                ])
                .with_security_scheme(SecuritySchemeConfig::basic("basicAuth"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // With invalid base64
    let resp = client
        .get(format!("http://{}/basic-protected", addr))
        .header("Authorization", "Basic !!!invalid-base64!!!")
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_basic_auth_www_authenticate_header() {
    use ultraapi::middleware::SecuritySchemeConfig;
    
    let app = UltraApiApp::new()
        .title("Basic Auth Test")
        .version("0.1.0")
        .basic_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_basic(vec![
                    ("admin".to_string(), "secret123".to_string()),
                ])
                .with_security_scheme(SecuritySchemeConfig::basic("basicAuth"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without credentials - should get WWW-Authenticate header
    let resp = reqwest::get(format!("http://{}/basic-protected", addr))
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 401);
    
    let www_auth = resp.headers().get("www-authenticate");
    assert!(www_auth.is_some());
    let www_auth_value = www_auth.unwrap().to_str().unwrap();
    assert!(www_auth_value.contains("Basic"));
    assert!(www_auth_value.contains("realm"));
}

#[tokio::test]
async fn test_basic_auth_openapi_spec() {
    use ultraapi::middleware::SecuritySchemeConfig;
    
    let app = UltraApiApp::new()
        .title("Basic Auth OpenAPI Test")
        .version("0.1.0")
        .basic_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_basic(vec![
                    ("admin".to_string(), "secret123".to_string()),
                ])
                .with_security_scheme(SecuritySchemeConfig::basic("basicAuth"))
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

    // Verify basicAuth security scheme is defined
    let schemes = body["components"]["securitySchemes"].as_object().unwrap();
    assert!(schemes.contains_key("basicAuth"));

    let basic = &schemes["basicAuth"];
    assert_eq!(basic["type"], "http");
    assert_eq!(basic["scheme"], "basic");

    // Check the route has security requirement
    let route = &body["paths"]["/basic-protected"]["get"];
    let security = route["security"].as_array().unwrap();
    assert!(security.iter().any(|s| s.get("basicAuth").is_some()));
}

// ============================================================================
// ApiError Format Tests (Ensure backward compatibility)
// ============================================================================

#[tokio::test]
async fn test_basic_auth_error_format() {
    use ultraapi::middleware::SecuritySchemeConfig;
    
    let app = UltraApiApp::new()
        .title("Basic Auth Error Format Test")
        .version("0.1.0")
        .basic_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_basic(vec![
                    ("admin".to_string(), "secret123".to_string()),
                ])
                .with_security_scheme(SecuritySchemeConfig::basic("basicAuth"))
        })
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    // Without credentials
    let resp = reqwest::get(format!("http://{}/basic-protected", addr))
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 401);
    
    // Check error format - should be ApiError: {error, details}
    let body: serde_json::Value = resp.json().await.unwrap();
    // Error should have "error" field (details may be empty array)
    assert!(body.get("error").is_some());
}
