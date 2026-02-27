//! Query Validation Tests
//!
//! Tests for Query extractor validation - ensuring validate() is called
//! when extracting Query<T> parameters.

use ultraapi::prelude::*;
use axum::extract::Query;

// --- Test Models ---

#[api_model]
#[derive(Debug, Clone)]
struct QueryValidationItem {
    #[validate(minimum = 1)]
    limit: i64,
}

/// Model with required field for OpenAPI required parameter testing
#[api_model]
#[derive(Debug, Clone)]
struct RequiredQueryModel {
    /// Required parameter
    id: i64,
    
    /// Optional parameter
    name: Option<String>,
}

// --- Test Routes ---

#[get("/__test_query_validation")]
async fn test_query_validation(query: Query<QueryValidationItem>) -> QueryValidationItem {
    query.0
}

#[get("/__test_required_query")]
async fn test_required_query(query: Query<RequiredQueryModel>) -> RequiredQueryModel {
    query.0
}

// --- Helper ---

fn create_query_validation_app() -> UltraApiApp {
    UltraApiApp::new()
        .title("Query Validation Test API")
        .version("0.1.0")
}

// --- Tests ---

#[tokio::test]
async fn test_query_validation_success() {
    let app = create_query_validation_app();
    let client = TestClient::new(app).await;
    
    // Valid query - limit=1 should pass
    let response = client.get("/__test_query_validation?limit=1").await;
    assert_eq!(response.status(), 200, "Valid query should return 200");
    
    let item: QueryValidationItem = response.json().await.unwrap();
    assert_eq!(item.limit, 1);
}

#[tokio::test]
async fn test_query_validation_failure() {
    let app = create_query_validation_app();
    let client = TestClient::new(app).await;
    
    // Invalid query - limit=0 should fail validation (minimum = 1)
    let response = client.get("/__test_query_validation?limit=0").await;
    assert_eq!(response.status(), 422, "Validation failure should return 422");
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"], "Validation failed", "Error message should be 'Validation failed'");
    assert!(body["details"].is_array(), "Details should be an array");
    assert!(!body["details"].as_array().unwrap().is_empty(), "Details should not be empty");
}

#[tokio::test]
async fn test_query_parse_failure() {
    let app = create_query_validation_app();
    let client = TestClient::new(app).await;
    
    // Invalid query - limit=notanumber should fail to parse (bad_request)
    let response = client.get("/__test_query_validation?limit=notanumber").await;
    assert_eq!(response.status(), 400, "Parse failure should return 400");
    
    let body: serde_json::Value = response.json().await.unwrap();
    // Should contain "Invalid query parameters" or similar
    assert!(body["error"].as_str().unwrap().to_lowercase().contains("query") || 
            body["error"].as_str().unwrap().to_lowercase().contains("invalid"),
            "Error should mention query/invalid");
}

#[tokio::test]
async fn test_query_validation_with_optional_field_failure() {
    let app = create_query_validation_app();
    let client = TestClient::new(app).await;
    
    // Invalid query - limit=0 should fail validation (minimum = 1)
    let response = client.get("/__test_query_validation?limit=0").await;
    assert_eq!(response.status(), 422, "Validation failure should return 422");
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"], "Validation failed");
    assert!(!body["details"].as_array().unwrap().is_empty());
}

/// Test OpenAPI required field validation at runtime (missing required parameter)
/// Note: Missing required fields cause parse errors (400) from axum, not validation errors (422)
/// This is because axum's Query extractor fails to deserialize when required fields are missing.
#[tokio::test]
async fn test_query_required_field_missing() {
    let app = create_query_validation_app();
    let client = TestClient::new(app).await;
    
    // Missing required parameter 'id' - axum returns 400 (parse error) for missing required fields
    let response = client.get("/__test_required_query?name=test").await;
    // Note: axum returns 400 for missing required query params
    assert_eq!(response.status(), 400, "Missing required field should return 400 (parse error)");
}

/// Test OpenAPI required field validation - providing all required fields
#[tokio::test]
async fn test_query_required_field_provided() {
    let app = create_query_validation_app();
    let client = TestClient::new(app).await;
    
    // Providing required parameter 'id'
    let response = client.get("/__test_required_query?id=1&name=test").await;
    assert_eq!(response.status(), 200, "Providing required field should return 200");
    
    let item: RequiredQueryModel = response.json().await.unwrap();
    assert_eq!(item.id, 1);
    assert_eq!(item.name, Some("test".to_string()));
}

/// Test validation error response format consistency with Body/Form
#[tokio::test]
async fn test_validation_error_format_consistency() {
    let app = create_query_validation_app();
    let client = TestClient::new(app).await;
    
    // Trigger validation failure
    let response = client.get("/__test_query_validation?limit=0").await;
    assert_eq!(response.status(), 422);
    
    let body: serde_json::Value = response.json().await.unwrap();
    
    // Check error format: { error: "Validation failed", details: [...] }
    assert!(body.get("error").is_some(), "Should have 'error' field");
    assert!(body.get("details").is_some(), "Should have 'details' field");
    assert_eq!(body["error"], "Validation failed");
    assert!(body["details"].is_array());
}
