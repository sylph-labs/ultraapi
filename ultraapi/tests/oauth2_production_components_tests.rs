// OAuth2 Production Components Tests
// Tests for OAuth2 types used in production (/token endpoint, validators)

use axum::http::StatusCode;
use ultraapi::prelude::*;

// ============================================================================
// OAuth2PasswordRequestForm Tests
// ============================================================================

#[test]
fn test_oauth2_password_request_form_basics() {
    // Test that OAuth2PasswordRequestForm can be created and used
    let form = OAuth2PasswordRequestForm {
        username: "user@example.com".to_string(),
        password: "secret123".to_string(),
        scope: "read write".to_string(),
        grant_type: "password".to_string(),
        client_id: Some("my-client-id".to_string()),
        client_secret: Some("my-client-secret".to_string()),
    };

    assert_eq!(form.username, "user@example.com");
    assert_eq!(form.password, "secret123");
    assert_eq!(form.client_id, Some("my-client-id".to_string()));
}

// ============================================================================
// TokenResponse Tests
// ============================================================================

#[test]
fn test_token_response_new() {
    let response = TokenResponse::new("test_token_123".to_string(), 3600);

    assert_eq!(response.access_token, "test_token_123");
    assert_eq!(response.token_type, "bearer");
    assert_eq!(response.expires_in, Some(3600));
    assert!(response.refresh_token.is_none());
}

#[test]
fn test_token_response_with_scopes() {
    let scopes = vec!["read".to_string(), "write".to_string()];
    let response = TokenResponse::with_scopes("test_token_123".to_string(), 3600, scopes.clone());

    assert_eq!(response.access_token, "test_token_123");
    assert_eq!(response.expires_in, Some(3600));
    assert_eq!(response.scope, "read write");
}

#[test]
fn test_token_response_with_refresh_token() {
    let response = TokenResponse::new("test_token_123".to_string(), 3600)
        .with_refresh_token("refresh_token_456".to_string());

    assert_eq!(
        response.refresh_token,
        Some("refresh_token_456".to_string())
    );
}

#[test]
fn test_token_response_json_serialization() {
    let response = TokenResponse::new("test_token_123".to_string(), 3600);
    let json = serde_json::to_string(&response).unwrap();

    assert!(json.contains("\"access_token\":\"test_token_123\""));
    assert!(json.contains("\"token_type\":\"bearer\""));
    assert!(json.contains("\"expires_in\":3600"));
}

#[test]
fn test_token_response_json_deserialization() {
    let json = r#"{"access_token":"test_token_123","token_type":"bearer","expires_in":3600}"#;
    let response: TokenResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.access_token, "test_token_123");
    assert_eq!(response.token_type, "bearer");
    assert_eq!(response.expires_in, Some(3600));
}

// ============================================================================
// OAuth2ErrorResponse Tests
// ============================================================================

#[test]
fn test_oauth2_error_response_invalid_request() {
    let error = OAuth2ErrorResponse::invalid_request("Missing required parameter");

    assert_eq!(error.error, "invalid_request");
    assert_eq!(
        error.error_description,
        Some("Missing required parameter".to_string())
    );
}

#[test]
fn test_oauth2_error_response_invalid_client() {
    let error = OAuth2ErrorResponse::invalid_client("Client authentication failed");

    assert_eq!(error.error, "invalid_client");
    assert_eq!(error.status_code(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_oauth2_error_response_invalid_grant() {
    let error = OAuth2ErrorResponse::invalid_grant("Invalid or expired token");

    assert_eq!(error.error, "invalid_grant");
    assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_oauth2_error_response_unsupported_grant_type() {
    let error = OAuth2ErrorResponse::unsupported_grant_type();

    assert_eq!(error.error, "unsupported_grant_type");
    assert!(error.error_description.is_some());
}

#[test]
fn test_oauth2_error_response_invalid_scope() {
    let error = OAuth2ErrorResponse::invalid_scope("Invalid scope requested");

    assert_eq!(error.error, "invalid_scope");
}

#[test]
fn test_oauth2_error_response_json_serialization() {
    let error = OAuth2ErrorResponse::invalid_request("Test error");
    let json = serde_json::to_string(&error).unwrap();

    assert!(json.contains("\"error\":\"invalid_request\""));
    assert!(json.contains("\"error_description\":\"Test error\""));
}

#[test]
fn test_oauth2_error_response_json_deserialization() {
    let json = r#"{"error":"invalid_request","error_description":"Test error"}"#;
    let error: OAuth2ErrorResponse = serde_json::from_str(json).unwrap();

    assert_eq!(error.error, "invalid_request");
    assert_eq!(error.error_description, Some("Test error".to_string()));
}

#[test]
fn test_oauth2_error_response_www_authenticate_header() {
    let error = OAuth2ErrorResponse::invalid_request("Test error");
    let header = error.www_authenticate_header();

    assert!(header.contains("Bearer"));
    assert!(header.contains("error=\"invalid_request\""));
    assert!(header.contains("error_description=\"Test error\""));
}

// ============================================================================
// TokenData Tests
// ============================================================================

#[test]
fn test_token_data_new() {
    let token_data = TokenData::new(
        "user123".to_string(),
        vec!["read".to_string(), "write".to_string()],
    );

    assert_eq!(token_data.sub, "user123");
    assert_eq!(token_data.scopes, vec!["read", "write"]);
    assert!(token_data.claims.is_empty());
}

#[test]
fn test_token_data_with_claim() {
    let token_data = TokenData::new("user123".to_string(), vec![]).with_claim(
        "email",
        serde_json::Value::String("user@example.com".to_string()),
    );

    assert!(token_data.claims.contains_key("email"));
    assert_eq!(
        token_data.claims["email"],
        serde_json::Value::String("user@example.com".to_string())
    );
}

#[test]
fn test_token_data_has_scope() {
    let token_data = TokenData::new(
        "user123".to_string(),
        vec!["read".to_string(), "write".to_string()],
    );

    assert!(token_data.has_scope("read"));
    assert!(token_data.has_scope("write"));
    assert!(!token_data.has_scope("admin"));
}

#[test]
fn test_token_data_has_all_scopes() {
    let token_data = TokenData::new(
        "user123".to_string(),
        vec!["read".to_string(), "write".to_string()],
    );

    assert!(token_data.has_all_scopes(&["read".to_string()]));
    assert!(token_data.has_all_scopes(&["read".to_string(), "write".to_string()]));
    assert!(!token_data.has_all_scopes(&["read".to_string(), "admin".to_string()]));
}

// ============================================================================
// OpaqueTokenValidator Tests
// ============================================================================

#[tokio::test]
async fn test_opaque_token_validator_valid_token() {
    let validator =
        OpaqueTokenValidator::new().add_token("valid-token-123", "user1", vec!["read".to_string()]);

    let result = validator.validate("valid-token-123").await;

    assert!(result.is_ok());
    let token_data = result.unwrap();
    assert_eq!(token_data.sub, "user1");
    assert_eq!(token_data.scopes, vec!["read"]);
}

#[tokio::test]
async fn test_opaque_token_validator_invalid_token() {
    let validator =
        OpaqueTokenValidator::new().add_token("valid-token-123", "user1", vec!["read".to_string()]);

    let result = validator.validate("invalid-token").await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        OAuth2AuthError::TokenNotFound
    ));
}

#[tokio::test]
async fn test_opaque_token_validator_validate_scopes_success() {
    let validator = OpaqueTokenValidator::new().add_token(
        "valid-token-123",
        "user1",
        vec!["read".to_string(), "write".to_string()],
    );

    let token_data = validator.validate("valid-token-123").await.unwrap();
    let result = validator.validate_scopes(&token_data, &["read".to_string()]);

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_opaque_token_validator_validate_scopes_failure() {
    let validator =
        OpaqueTokenValidator::new().add_token("valid-token-123", "user1", vec!["read".to_string()]);

    let token_data = validator.validate("valid-token-123").await.unwrap();
    let result = validator.validate_scopes(&token_data, &["read".to_string(), "admin".to_string()]);

    assert!(result.is_err());
    if let Err(OAuth2AuthError::InsufficientScope { required, provided }) = result {
        assert_eq!(required, vec!["read", "admin"]);
        assert_eq!(provided, vec!["read"]);
    } else {
        panic!("Expected InsufficientScope error");
    }
}

#[tokio::test]
async fn test_opaque_token_validator_extend_tokens() {
    let validator = OpaqueTokenValidator::new().extend_tokens([
        ("token1", "user1", vec!["read".to_string()]),
        ("token2", "user2", vec!["write".to_string()]),
    ]);

    let result1 = validator.validate("token1").await;
    let result2 = validator.validate("token2").await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert_eq!(result1.unwrap().sub, "user1");
    assert_eq!(result2.unwrap().sub, "user2");
}

#[tokio::test]
async fn test_opaque_token_validator_remove_token() {
    let validator =
        OpaqueTokenValidator::new().add_token("token1", "user1", vec!["read".to_string()]);

    let validator2 = validator.remove_token("token1");
    let result = validator2.validate("token1").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_opaque_token_validator_clone() {
    let validator1 =
        OpaqueTokenValidator::new().add_token("token1", "user1", vec!["read".to_string()]);

    let validator2 = validator1.clone();

    let result = validator2.validate("token1").await;
    assert!(result.is_ok());
}

// ============================================================================
// OAuth2AuthError Tests
// ============================================================================

#[test]
fn test_oauth2_auth_error_display() {
    let error = OAuth2AuthError::InvalidToken("Token is malformed".to_string());
    assert_eq!(error.to_string(), "Invalid token: Token is malformed");

    let error = OAuth2AuthError::ExpiredToken;
    assert_eq!(error.to_string(), "Token has expired");

    let error = OAuth2AuthError::TokenNotFound;
    assert_eq!(error.to_string(), "Token not found");

    let error = OAuth2AuthError::InsufficientScope {
        required: vec!["read".to_string(), "write".to_string()],
        provided: vec!["read".to_string()],
    };
    assert!(error.to_string().contains("Insufficient scope"));
}

#[test]
fn test_oauth2_auth_error_serialization() {
    let error = OAuth2AuthError::InvalidToken("Test error".to_string());
    let json = serde_json::to_string(&error).unwrap();

    assert!(json.contains("Invalid token: Test error"));
}

// ============================================================================
// OAuth2PasswordRequestForm Tests
// ============================================================================

#[test]
fn test_oauth2_password_request_form_scopes() {
    let form = OAuth2PasswordRequestForm {
        username: "user@example.com".to_string(),
        password: "secret123".to_string(),
        scope: "read write admin".to_string(),
        grant_type: "password".to_string(),
        client_id: None,
        client_secret: None,
    };

    let scopes = form.scopes();
    assert_eq!(scopes, vec!["read", "write", "admin"]);
}

#[test]
fn test_oauth2_password_request_form_empty_scope() {
    let form = OAuth2PasswordRequestForm {
        username: "user@example.com".to_string(),
        password: "secret123".to_string(),
        scope: "".to_string(),
        grant_type: "password".to_string(),
        client_id: None,
        client_secret: None,
    };

    let scopes = form.scopes();
    assert!(scopes.is_empty());
}

#[test]
fn test_oauth2_password_request_form_is_password_grant() {
    let form = OAuth2PasswordRequestForm {
        username: "user@example.com".to_string(),
        password: "secret123".to_string(),
        scope: "".to_string(),
        grant_type: "password".to_string(),
        client_id: None,
        client_secret: None,
    };

    assert!(form.is_password_grant());
}

#[test]
fn test_oauth2_password_request_form_is_password_grant_empty() {
    let form = OAuth2PasswordRequestForm {
        username: "user@example.com".to_string(),
        password: "secret123".to_string(),
        scope: "".to_string(),
        grant_type: "".to_string(),
        client_id: None,
        client_secret: None,
    };

    assert!(form.is_password_grant());
}

#[test]
fn test_oauth2_password_request_form_not_password_grant() {
    let form = OAuth2PasswordRequestForm {
        username: "user@example.com".to_string(),
        password: "secret123".to_string(),
        scope: "".to_string(),
        grant_type: "client_credentials".to_string(),
        client_id: None,
        client_secret: None,
    };

    assert!(!form.is_password_grant());
}

// ============================================================================
// Integration: Token Endpoint Example
// ============================================================================

#[tokio::test]
async fn test_token_endpoint_example() {
    // This test demonstrates a complete token endpoint implementation

    // Setup validator with test tokens
    let _validator = OpaqueTokenValidator::new().add_token(
        "user_pass_valid",
        "user1",
        vec!["read".to_string(), "write".to_string()],
    );

    // Simulate password grant request
    let form = OAuth2PasswordRequestForm {
        username: "user@example.com".to_string(),
        password: "password123".to_string(),
        scope: "read write".to_string(),
        grant_type: "password".to_string(),
        client_id: None,
        client_secret: None,
    };

    // In a real implementation, you would validate credentials against a user store
    // For this test, we simulate successful authentication with a valid token

    // Validate the token (in real app, you'd check password hash)
    // Here we just check if it's password grant type
    assert!(form.is_password_grant());

    // Generate token response
    let response =
        TokenResponse::with_scopes("generated_token_abc".to_string(), 3600, form.scopes());

    assert_eq!(response.access_token, "generated_token_abc");
    assert_eq!(response.expires_in, Some(3600));
    assert_eq!(response.scope, "read write");
}

#[tokio::test]
async fn test_token_endpoint_invalid_grant() {
    // Test error response for invalid grant

    let error = OAuth2ErrorResponse::invalid_grant("Invalid username or password");

    assert_eq!(error.error, "invalid_grant");
    assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);

    let json = serde_json::to_string(&error).unwrap();
    assert!(json.contains("invalid_grant"));
}

// ============================================================================
// Prelude Export Tests
// ============================================================================

#[test]
fn test_prelude_exports_oauth2_types() {
    // Verify that prelude exports the OAuth2 types
    // This is a compile-time test - if it compiles, the exports are correct

    // These types should be accessible from prelude
    let _form = OAuth2PasswordRequestForm {
        username: "test".to_string(),
        password: "test".to_string(),
        scope: "".to_string(),
        grant_type: "password".to_string(),
        client_id: None,
        client_secret: None,
    };

    let _response = TokenResponse::new("token".to_string(), 3600);

    let _error = OAuth2ErrorResponse::invalid_request("test");

    let _token_data = TokenData::new("sub".to_string(), vec![]);

    let _validator = OpaqueTokenValidator::new();
}

// ============================================================================
// OAuth2 Module Export Tests
// ============================================================================

#[test]
fn test_oauth2_module_exports() {
    // Verify that oauth2 module exports are available
    use ultraapi::oauth2::{OAuth2ErrorResponse, OAuth2PasswordRequestForm, TokenResponse};

    let _form = OAuth2PasswordRequestForm {
        username: "test".to_string(),
        password: "test".to_string(),
        scope: "".to_string(),
        grant_type: "password".to_string(),
        client_id: None,
        client_secret: None,
    };

    let _response = TokenResponse::new("token".to_string(), 3600);

    let _error = OAuth2ErrorResponse::invalid_request("test");
}
