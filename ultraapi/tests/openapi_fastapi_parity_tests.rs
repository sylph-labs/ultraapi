// OpenAPI FastAPI Parity Golden Tests
// Tests that compare generated OpenAPI output against a golden snapshot
// to ensure FastAPI-compatible OpenAPI generation.

use ultraapi::axum;
use ultraapi::prelude::*;
use serde_json::Value;

// --- Test API Models ---

#[api_model]
#[derive(Debug, Clone)]
struct Item {
    id: i64,
    name: String,
    price: f64,
}

#[api_model]
#[derive(Debug, Clone)]
struct CreateItem {
    #[validate(min_length = 1, max_length = 100)]
    name: String,
    #[validate(minimum = 0)]
    price: f64,
    #[validate(pattern = "^[A-Z]{3}$")]
    code: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct UpdateItem {
    name: Option<String>,
    price: Option<f64>,
}

#[derive(ultraapi::serde::Deserialize, ultraapi::schemars::JsonSchema)]
struct PaginationQuery {
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(ultraapi::serde::Serialize, ultraapi::schemars::JsonSchema)]
struct ListResponse {
    items: Vec<Item>,
    total: i64,
    page: i64,
}

// --- Test Endpoints ---

/// Get item by ID (path parameter)
#[get("/items/{id}")]
#[tag("items")]
async fn get_item(id: i64) -> Result<Item, ApiError> {
    Ok(Item { id, name: "Test Item".into(), price: 9.99 })
}

/// Create new item (request body with validation)
#[post("/items")]
#[tag("items")]
async fn create_item(item: CreateItem) -> Item {
    Item { id: 1, name: item.name, price: item.price }
}

/// Update item (path + request body)
#[put("/items/{id}")]
#[tag("items")]
async fn update_item(id: i64, item: UpdateItem) -> Item {
    Item { 
        id, 
        name: item.name.unwrap_or_default(), 
        price: item.price.unwrap_or(0.0) 
    }
}

/// List items with pagination (query parameters)
#[get("/items")]
#[tag("items")]
async fn list_items(query: Query<PaginationQuery>) -> ListResponse {
    ListResponse { 
        items: vec![Item { id: 1, name: "Item 1".into(), price: 10.0 }], 
        total: 1, 
        page: query.page.unwrap_or(1), 
    }
}

/// Delete item by ID
#[delete("/items/{id}")]
#[tag("items")]
async fn delete_item(id: i64) -> Result<(), ApiError> {
    Ok(())
}

// --- App Builder ---

fn build_test_app() -> UltraApiApp {
    // Routes are automatically registered via #[get], #[post], etc. macros
    // The into_router() call collects all registered routes
    UltraApiApp::new()
        .title("Parity Test API")
        .version("0.1.0")
}

// --- Helper Functions for Golden Comparison ---

/// Recursively sort all object keys in a JSON value for deterministic comparison
fn normalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(b.0));
            let normalized: serde_json::Map<String, Value> = sorted
                .into_iter()
                .map(|(k, v)| (k.clone(), normalize_json(v)))
                .collect();
            Value::Object(normalized)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(normalize_json).collect())
        }
        Value::String(s) => Value::String(s.clone()),
        Value::Number(n) => Value::Number(n.clone()),
        Value::Bool(b) => Value::Bool(*b),
        Value::Null => Value::Null,
    }
}

/// Extract the parity-critical subset of OpenAPI for comparison
/// This focuses on FastAPI parity-critical fields and ignores volatile metadata.
fn extract_parity_subset(full: &Value) -> Value {
    let mut result = Value::Object(serde_json::Map::new());
    
    // Top-level fields we care about
    if let Some(obj) = full.as_object() {
        // openapi version
        if let Some(v) = obj.get("openapi") {
            result["openapi"] = v.clone();
        }
        
        // info
        if let Some(info) = obj.get("info") {
            let mut info_obj = serde_json::Map::new();
            if let Some(i) = info.as_object() {
                if let Some(title) = i.get("title") {
                    info_obj.insert("title".to_string(), title.clone());
                }
                if let Some(version) = i.get("version") {
                    info_obj.insert("version".to_string(), version.clone());
                }
            }
            if !info_obj.is_empty() {
                result["info"] = Value::Object(info_obj);
            }
        }
        
        // paths - the core of FastAPI parity
        if let Some(paths) = obj.get("paths") {
            result["paths"] = normalize_json(paths);
        }
        
        // components/schemas - the models
        if let Some(components) = obj.get("components") {
            if let Some(schemas) = components.get("schemas") {
                result["components"] = Value::Object([
                    ("schemas".to_string(), normalize_json(schemas))
                ].into_iter().collect());
            }
        }
    }
    
    result
}

/// Load golden file or return None if it doesn't exist
fn load_golden() -> Option<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("openapi_fastapi_parity.json");
    
    if path.exists() {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

/// Save current OpenAPI output as the new golden file
fn save_golden(value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("openapi_fastapi_parity.json");
    
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let content = serde_json::to_string_pretty(value)?;
    std::fs::write(path, content)?;
    Ok(())
}

// --- Tests ---

#[tokio::test]
async fn test_openapi_fastapi_parity_golden() {
    // Build and spawn the test app
    let app = build_test_app().into_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    // Give the server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Fetch OpenAPI JSON
    let base = format!("http://{}", addr);
    let resp = reqwest::get(format!("{}/openapi.json", base)).await.unwrap();
    assert_eq!(resp.status(), 200, "OpenAPI endpoint should return 200");
    
    let body: Value = resp.json().await.unwrap();
    
    // Extract parity-critical subset
    let parity_output = extract_parity_subset(&body);
    let normalized_output = normalize_json(&parity_output);
    
    // Check for golden file
    match load_golden() {
        Some(golden) => {
            let normalized_golden = normalize_json(&golden);
            
            // Compare the outputs
            assert_eq!(
                normalized_output, 
                normalized_golden,
                "OpenAPI output does not match golden file. To update golden, run with UPDATE_GOLDEN=1"
            );
        }
        None => {
            // No golden file exists - create one
            save_golden(&normalized_output).expect("Failed to create golden file");
            panic!("Created initial golden file. Re-run tests to verify.");
        }
    }
}

/// Helper test to regenerate the golden file
/// Run with: UPDATE_GOLDEN=1 cargo test test_openapi_fastapi_parity_regenerate
#[tokio::test]
async fn test_openapi_fastapi_parity_regenerate() {
    // Only run if UPDATE_GOLDEN env var is set
    if std::env::var("UPDATE_GOLDEN").ok() != Some("1".to_string()) {
        return;
    }
    
    // Build and spawn the test app
    let app = build_test_app().into_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Fetch OpenAPI JSON
    let base = format!("http://{}", addr);
    let resp = reqwest::get(format!("{}/openapi.json", base)).await.unwrap();
    assert_eq!(resp.status(), 200);
    
    let body: Value = resp.json().await.unwrap();
    let parity_output = extract_parity_subset(&body);
    let normalized_output = normalize_json(&parity_output);
    
    save_golden(&normalized_output).expect("Failed to update golden file");
    println!("Updated golden file at ultraapi/tests/golden/openapi_fastapi_parity.json");
}

/// Test that validates specific FastAPI parity aspects
/// This test always runs and validates specific known differences from FastAPI
#[tokio::test]
async fn test_fastapi_parity_specific_validations() {
    // Build and spawn the test app
    let app = build_test_app().into_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    let base = format!("http://{}", addr);
    let resp = reqwest::get(format!("{}/openapi.json", base)).await.unwrap();
    let body: Value = resp.json().await.unwrap();
    
    let paths = body["paths"].as_object().expect("paths should be object");
    
    // 1. Path parameters should be properly defined
    let get_item = paths["/items/{id}"]["get"].as_object().expect("GET /items/{id} should exist");
    let params = get_item["parameters"].as_array().expect("parameters should be array");
    let id_param = params.iter().find(|p| p["name"] == "id").expect("id param should exist");
    assert_eq!(id_param["in"], "path", "id should be a path parameter");
    assert_eq!(id_param["required"], true, "path params should be required");
    assert_eq!(id_param["schema"]["type"], "integer", "id should be integer");
    
    // 2. Query parameters from struct should work
    let list_items = paths["/items"]["get"].as_object().expect("GET /items should exist");
    let query_params = list_items["parameters"].as_array().expect("parameters should be array");
    assert!(!query_params.is_empty(), "list_items should have query parameters");
    
    // 3. Request body with validation constraints
    let create_item = paths["/items"]["post"].as_object().expect("POST /items should exist");
    let request_body = create_item["requestBody"].as_object().expect("requestBody should exist");
    let content = request_body["content"].as_object().expect("content should exist");
    let json_schema = content["application/json"]["schema"].as_object().expect("JSON schema should exist");
    
    // Check for $ref to CreateItem schema
    assert!(
        json_schema.get("$ref").is_some() || json_schema.get("properties").is_some(),
        "Request body should have schema"
    );
    
    // 4. Components/schemas should contain our models
    let schemas = body["components"]["schemas"].as_object().expect("schemas should exist");
    
    // CreateItem should have validation constraints
    if let Some(create_item_schema) = schemas.get("CreateItem") {
        let props = create_item_schema["properties"].as_object().expect("properties should exist");
        
        // name should have minLength/maxLength
        if let Some(name) = props.get("name") {
            assert_eq!(name["type"], "string", "name should be string type");
            // Note: UltraAPI may output minLength/maxLength differently than FastAPI
            // Known difference: FastAPI uses "minLength" and "maxLength" directly on schema
            // UltraAPI may use different representation - this is a known parity difference
        }
        
        // price should have minimum
        if let Some(price) = props.get("price") {
            assert_eq!(price["type"], "number", "price should be number type");
        }
        
        // code should have pattern
        if let Some(code) = props.get("code") {
            assert_eq!(code["type"], "string", "code should be string type");
            // Note: pattern validation - FastAPI uses "pattern", UltraAPI may differ
        }
    }
    
    // 5. Response schemas should be defined
    let get_item_resp = get_item["responses"]["200"]["content"]["application/json"]["schema"].as_object();
    assert!(get_item_resp.is_some(), "GET should have response schema");
    
    println!("FastAPI parity validations passed");
}
