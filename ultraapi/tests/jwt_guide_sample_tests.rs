// JWT guide sample tests
//
// These tests document how to integrate a JWT-like validator with AuthLayer.

use ultraapi::prelude::*;
use ultraapi::middleware::{AuthError, AuthValidator, Credentials, SecuritySchemeConfig};

#[get("/jwt-protected")]
#[security("bearer")]
async fn jwt_protected() -> String {
    "ok".to_string()
}

#[derive(Clone)]
struct DemoJwtValidator;

impl AuthValidator for DemoJwtValidator {
    fn validate(&self, credentials: &Credentials) -> Result<(), AuthError> {
        if credentials.scheme.to_lowercase() != "bearer" {
            return Err(AuthError::unauthorized("Invalid auth scheme"));
        }

        match credentials.value.as_str() {
            "valid-read" | "valid-write" => Ok(()),
            _ => Err(AuthError::unauthorized("Invalid or expired token")),
        }
    }

    fn validate_scopes(
        &self,
        credentials: &Credentials,
        required_scopes: &[String],
    ) -> Result<(), AuthError> {
        if required_scopes.is_empty() {
            return Ok(());
        }

        // Demo: token determines scopes
        let scopes: Vec<&str> = match credentials.value.as_str() {
            "valid-read" => vec!["read"],
            "valid-write" => vec!["write"],
            _ => vec![],
        };

        if required_scopes.iter().all(|s| scopes.contains(&s.as_str())) {
            Ok(())
        } else {
            Err(AuthError::forbidden("Insufficient scope"))
        }
    }
}

#[tokio::test]
async fn test_jwt_validator_allows_valid_token() {
    let app = UltraApiApp::new()
        .title("JWT")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_validator(DemoJwtValidator)
                .with_security_scheme(SecuritySchemeConfig::bearer("bearerAuth"))
        })
        .include(UltraApiRouter::new("").route(__HAYAI_ROUTE_JWT_PROTECTED))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/jwt-protected", addr))
        .header("Authorization", "Bearer valid-read")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_jwt_validator_missing_token_returns_401() {
    let app = UltraApiApp::new()
        .title("JWT")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_validator(DemoJwtValidator)
                .with_security_scheme(SecuritySchemeConfig::bearer("bearerAuth"))
        })
        .include(UltraApiRouter::new("").route(__HAYAI_ROUTE_JWT_PROTECTED))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let resp = reqwest::get(format!("http://{}/jwt-protected", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
    assert!(resp.headers().get("www-authenticate").is_some());
}

#[tokio::test]
async fn test_jwt_validator_scope_forbidden_returns_403() {
    let app = UltraApiApp::new()
        .title("JWT")
        .version("0.1.0")
        .bearer_auth()
        .middleware(|builder| {
            builder
                .enable_auth_with_validator(DemoJwtValidator)
                .with_security_scheme(
                    SecuritySchemeConfig::bearer("bearerAuth").with_scopes(vec!["read".to_string()]),
                )
        })
        .include(UltraApiRouter::new("").route(__HAYAI_ROUTE_JWT_PROTECTED))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/jwt-protected", addr))
        .header("Authorization", "Bearer valid-write")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
}
