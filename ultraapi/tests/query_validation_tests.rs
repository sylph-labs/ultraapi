//! Query Validation Tests
//!
//! Tests for Query extractor validation - ensuring validate() is called
//! when extracting Query<T> parameters.

use ultraapi::prelude::*;

// --- Test Models ---

#[api_model]
#[derive(Debug, Clone)]
struct QueryValidationItem {
    #[validate(minimum = 1)]
    limit: i64,
}

// --- Test Routes ---

#[get("/__test_query_validation")]
async fn test_query_validation(query: QueryValidationItem) -> QueryValidationItem {
    query
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
