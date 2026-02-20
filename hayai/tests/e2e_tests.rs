use hayai::prelude::*;
use hayai::axum;
use serde_json::Value;

// --- App setup ---

#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct CreateUser {
    #[validate(min_length = 1, max_length = 100)]
    name: String,
    #[validate(email)]
    email: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct Address {
    city: String,
    country: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct UserWithAddress {
    name: String,
    address: Address,
    tags: Vec<String>,
    nickname: Option<String>,
}

#[derive(hayai::serde::Deserialize, hayai::schemars::JsonSchema)]
struct Pagination {
    page: Option<i64>,
    limit: Option<i64>,
}

struct Database;
impl Database {
    async fn get_user(&self, id: i64) -> Option<User> {
        if id == 9999 { return None; }
        Some(User { id, name: "Alice".into(), email: "alice@example.com".into() })
    }
    async fn create_user(&self, input: &CreateUser) -> User {
        User { id: 1, name: input.name.clone(), email: input.email.clone() }
    }
    async fn list_users(&self, _page: Option<i64>, _limit: Option<i64>) -> Vec<User> {
        vec![User { id: 1, name: "Alice".into(), email: "alice@example.com".into() }]
    }
}

/// Get a user by ID
#[get("/users/{id}")]
#[tag("users")]
async fn get_user(id: i64, db: Dep<Database>) -> Result<User, ApiError> {
    db.get_user(id).await
        .ok_or_else(|| ApiError::not_found(format!("User {} not found", id)))
}

/// Create a new user
#[post("/users")]
#[tag("users")]
async fn create_user(body: CreateUser, db: Dep<Database>) -> User {
    db.create_user(&body).await
}

/// List users with pagination
#[get("/users")]
#[tag("users")]
async fn list_users(query: Query<Pagination>, db: Dep<Database>) -> Vec<User> {
    db.list_users(query.page, query.limit).await
}

// --- Helper ---

async fn spawn_app() -> String {
    let app = HayaiApp::new()
        .title("Test API")
        .version("0.1.0")
        .dep(Database)
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

// --- Tests ---

#[tokio::test]
async fn test_get_user_returns_200() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/users/42")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 42);
    assert_eq!(body["name"], "Alice");
    assert_eq!(body["email"], "alice@example.com");
}

#[tokio::test]
async fn test_create_user_valid() {
    let base = spawn_app().await;
    let client = reqwest::Client::new();
    let resp = client.post(format!("{base}/users"))
        .json(&serde_json::json!({"name": "Bob", "email": "bob@example.com"}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Bob");
    assert_eq!(body["email"], "bob@example.com");
}

#[tokio::test]
async fn test_create_user_empty_name_returns_422() {
    let base = spawn_app().await;
    let client = reqwest::Client::new();
    let resp = client.post(format!("{base}/users"))
        .json(&serde_json::json!({"name": "", "email": "bob@example.com"}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "Validation failed");
    assert!(!body["details"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_create_user_invalid_email_returns_422() {
    let base = spawn_app().await;
    let client = reqwest::Client::new();
    let resp = client.post(format!("{base}/users"))
        .json(&serde_json::json!({"name": "Bob", "email": "notanemail"}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    let details = body["details"].as_array().unwrap();
    assert!(details.iter().any(|d| d.as_str().unwrap().contains("email")));
}

#[tokio::test]
async fn test_create_user_malformed_json_returns_400() {
    let base = spawn_app().await;
    let client = reqwest::Client::new();
    let resp = client.post(format!("{base}/users"))
        .header("content-type", "application/json")
        .body("{not json")
        .send().await.unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_openapi_spec() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["openapi"], "3.1.0");
    assert!(body["paths"].as_object().unwrap().contains_key("/users/{id}"));
    assert!(body["paths"].as_object().unwrap().contains_key("/users"));
    assert!(body["components"]["schemas"].as_object().unwrap().contains_key("User"));
}

#[tokio::test]
async fn test_openapi_nested_schemas() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();
    let schemas = &body["components"]["schemas"];
    
    // Address should exist as a nested schema
    assert!(schemas.get("Address").is_some(), "Address schema should be in components/schemas");
    assert_eq!(schemas["Address"]["type"], "object");
    assert!(schemas["Address"]["properties"]["city"]["type"] == "string");
    
    // UserWithAddress should have $ref for address
    assert!(schemas.get("UserWithAddress").is_some());
    assert_eq!(schemas["UserWithAddress"]["properties"]["address"]["$ref"], "#/components/schemas/Address");
    
    // tags should be array
    assert_eq!(schemas["UserWithAddress"]["properties"]["tags"]["type"], "array");
    assert_eq!(schemas["UserWithAddress"]["properties"]["tags"]["items"]["type"], "string");
    
    // nickname should be nullable (anyOf)
    assert!(schemas["UserWithAddress"]["properties"]["nickname"].get("anyOf").is_some());
}

#[tokio::test]
async fn test_docs_returns_swagger_html() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/docs")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.to_lowercase().contains("swagger"));
}

#[tokio::test]
async fn test_nonexistent_route_returns_404() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/nonexistent")).await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_dep_injection_works() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/users/1")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 1);
}

// ---- Query Parameter E2E Tests ----

#[tokio::test]
async fn test_list_users_with_query_params() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/users?page=1&limit=10")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body.as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_list_users_without_query_params() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/users")).await.unwrap();
    assert_eq!(resp.status(), 200);
}

// ---- OpenAPI Spec: Error Responses ----

#[tokio::test]
async fn test_openapi_error_responses() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    // Check ApiError schema exists
    assert!(body["components"]["schemas"]["ApiError"].is_object());

    // Check POST /users has 400 and 422 error responses
    let post_users = &body["paths"]["/users"]["post"];
    assert!(post_users["responses"]["400"].is_object(), "should have 400 response");
    assert!(post_users["responses"]["422"].is_object(), "should have 422 for body endpoints");
    assert!(post_users["responses"]["500"].is_object(), "should have 500 response");

    // GET endpoints should have 400 and 500 but not 422
    let get_user = &body["paths"]["/users/{id}"]["get"];
    assert!(get_user["responses"]["400"].is_object());
    assert!(get_user["responses"]["500"].is_object());
    assert!(!get_user["responses"]["422"].is_object(), "GET without body should not have 422");
}

// ---- OpenAPI Spec: Status Codes ----

#[tokio::test]
async fn test_openapi_status_codes() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    // POST /users should have 201 (default for POST)
    let post_users = &body["paths"]["/users"]["post"];
    assert!(post_users["responses"]["201"].is_object(), "POST should default to 201");

    // GET /users/{id} should have 200 (default for GET)
    let get_user = &body["paths"]["/users/{id}"]["get"];
    assert!(get_user["responses"]["200"].is_object(), "GET should default to 200");
}

// ---- OpenAPI Spec: Tags ----

#[tokio::test]
async fn test_openapi_tags() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    let get_user = &body["paths"]["/users/{id}"]["get"];
    let tags = get_user["tags"].as_array().unwrap();
    assert!(tags.iter().any(|t| t == "users"));
}

// ---- OpenAPI Spec: Descriptions ----

#[tokio::test]
async fn test_openapi_descriptions() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    let get_user = &body["paths"]["/users/{id}"]["get"];
    assert_eq!(get_user["description"], "Get a user by ID");

    let create_user = &body["paths"]["/users"]["post"];
    assert_eq!(create_user["description"], "Create a new user");
}

// ---- OpenAPI Spec: Query Parameters ----

#[tokio::test]
async fn test_openapi_query_params() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    let list_users = &body["paths"]["/users"]["get"];
    let params = list_users["parameters"].as_array().unwrap();
    let names: Vec<&str> = params.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"page"), "should have page query param");
    assert!(names.contains(&"limit"), "should have limit query param");
    for p in params {
        assert_eq!(p["in"], "query");
        assert_eq!(p["required"], false, "Optional query params should not be required");
    }
}

// ---- Vec<T> Response E2E ----

#[tokio::test]
async fn test_openapi_vec_response_schema() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    // GET /users returns Vec<User>, should have array schema
    let get_users = &body["paths"]["/users"]["get"];
    let success_resp = &get_users["responses"]["200"];
    let schema = &success_resp["content"]["application/json"]["schema"];
    assert_eq!(schema["type"], "array", "Vec<T> response should be array type");
    assert_eq!(schema["items"]["$ref"], "#/components/schemas/User", "Array items should $ref User");
}

// ---- Enum $ref E2E ----

// Note: The example app has User.status field which is Status enum
// This test would need a model with an enum field registered - covered by unit tests above

// ---- Response Description E2E ----

#[tokio::test]
async fn test_openapi_response_descriptions() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    let get_user = &body["paths"]["/users/{id}"]["get"];
    assert_eq!(get_user["responses"]["200"]["description"], "OK");

    let post_user = &body["paths"]["/users"]["post"];
    assert_eq!(post_user["responses"]["201"]["description"], "Created");
}

// ===== Router E2E tests =====

#[api_model]
#[derive(Debug, Clone)]
struct Item {
    id: i64,
    name: String,
}

#[get("/e2e-rt-list")]
async fn e2e_list_items() -> Vec<Item> {
    vec![Item { id: 1, name: "Widget".into() }]
}

#[get("/e2e-rt-item/{id}")]
async fn e2e_get_item(id: i64) -> Item {
    Item { id, name: "Widget".into() }
}

#[delete("/e2e-rt-del/{id}")]
async fn e2e_delete_item(id: i64) -> () {
    let _ = id;
}

async fn spawn_router_app() -> String {
    let items = hayai::HayaiRouter::new("/api/items")
        .tag("items")
        .security("bearer")
        .route(__HAYAI_ROUTE_E2E_LIST_ITEMS)
        .route(__HAYAI_ROUTE_E2E_GET_ITEM)
        .route(__HAYAI_ROUTE_E2E_DELETE_ITEM);

    let app = HayaiApp::new()
        .title("Router Test API")
        .version("0.1.0")
        .bearer_auth()
        .include(items)
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_router_e2e_get_prefixed_path() {
    let base = spawn_router_app().await;
    let resp = reqwest::get(format!("{base}/api/items/e2e-rt-item/42")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 42);
    assert_eq!(body["name"], "Widget");
}

#[tokio::test]
async fn test_router_e2e_list_prefixed_path() {
    let base = spawn_router_app().await;
    let resp = reqwest::get(format!("{base}/api/items/e2e-rt-list")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body.as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_router_e2e_delete_returns_204() {
    let base = spawn_router_app().await;
    let client = reqwest::Client::new();
    let resp = client.delete(format!("{base}/api/items/e2e-rt-del/1")).send().await.unwrap();
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn test_router_e2e_openapi_prefixed_paths() {
    let base = spawn_router_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    // Paths should be prefixed
    assert!(body["paths"]["/api/items/e2e-rt-list"]["get"].is_object());
    assert!(body["paths"]["/api/items/e2e-rt-item/{id}"]["get"].is_object());

    // Tags should include router-level tag
    let tags = body["paths"]["/api/items/e2e-rt-list"]["get"]["tags"].as_array().unwrap();
    assert!(tags.iter().any(|t| t == "items"));

    // Security should include router-level security
    let sec = body["paths"]["/api/items/e2e-rt-list"]["get"]["security"].as_array().unwrap();
    assert!(sec.iter().any(|s| s.get("bearerAuth").is_some()));
}

#[tokio::test]
async fn test_router_e2e_original_path_not_registered() {
    let base = spawn_router_app().await;
    // The original unprefixed path should NOT exist
    let resp = reqwest::get(format!("{base}/e2e-rt-list")).await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_router_e2e_nested_routers() {
    let items = hayai::HayaiRouter::new("/items")
        .route(__HAYAI_ROUTE_E2E_LIST_ITEMS);
    let v1 = hayai::HayaiRouter::new("/v1")
        .include(items);
    let api = hayai::HayaiRouter::new("/api")
        .include(v1);

    let app = HayaiApp::new()
        .include(api)
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let base = format!("http://{}", addr);

    let resp = reqwest::get(format!("{base}/api/v1/items/e2e-rt-list")).await.unwrap();
    assert_eq!(resp.status(), 200);
}

// ===== Result<T, ApiError> E2E tests =====

#[tokio::test]
async fn test_result_handler_ok_returns_200() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/users/42")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 42);
    assert_eq!(body["name"], "Alice");
}

#[tokio::test]
async fn test_result_handler_not_found_returns_404() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/users/9999")).await.unwrap();
    assert_eq!(resp.status(), 404);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_result_handler_bad_request_returns_400() {
    // Test a Result-returning handler that returns bad_request error
    let base = spawn_result_app().await;
    let resp = reqwest::get(format!("{base}/result-test/bad")).await.unwrap();
    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("bad input"));
}

#[tokio::test]
async fn test_openapi_result_handler_has_404_response() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();

    // GET /users/{id} returns Result, should have 404 response
    let get_user = &body["paths"]["/users/{id}"]["get"];
    assert!(get_user["responses"]["404"].is_object(), "Result handler should have 404 response");
    assert_eq!(get_user["responses"]["404"]["description"], "Not Found");
}

// ===== PUT E2E tests =====

#[api_model]
#[derive(Debug, Clone)]
struct UpdateItem {
    #[validate(min_length = 1)]
    name: String,
}

#[put("/e2e-put-item/{id}")]
async fn e2e_put_item(id: i64, body: UpdateItem) -> Item {
    Item { id, name: body.name }
}

#[get("/result-test/{mode}")]
async fn e2e_result_test(mode: String) -> Result<Item, ApiError> {
    match mode.as_str() {
        "bad" => Err(ApiError::bad_request("bad input".to_string())),
        _ => Ok(Item { id: 1, name: "ok".into() }),
    }
}

async fn spawn_result_app() -> String {
    let app = HayaiApp::new()
        .title("Result Test API")
        .version("0.1.0")
        .include(
            hayai::HayaiRouter::new("")
                .route(__HAYAI_ROUTE_E2E_RESULT_TEST)
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_put_app() -> String {
    let app = HayaiApp::new()
        .title("PUT Test API")
        .version("0.1.0")
        .include(
            hayai::HayaiRouter::new("/api")
                .route(__HAYAI_ROUTE_E2E_PUT_ITEM)
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_put_returns_200() {
    let base = spawn_put_app().await;
    let client = reqwest::Client::new();
    let resp = client.put(format!("{base}/api/e2e-put-item/5"))
        .json(&serde_json::json!({"name": "Updated"}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 5);
    assert_eq!(body["name"], "Updated");
}

#[tokio::test]
async fn test_put_validation_error_returns_422() {
    let base = spawn_put_app().await;
    let client = reqwest::Client::new();
    let resp = client.put(format!("{base}/api/e2e-put-item/5"))
        .json(&serde_json::json!({"name": ""}))
        .send().await.unwrap();
    assert_eq!(resp.status(), 422);
}

#[tokio::test]
async fn test_put_openapi_spec() {
    let base = spawn_put_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    let body: Value = resp.json().await.unwrap();
    assert!(body["paths"]["/api/e2e-put-item/{id}"]["put"].is_object(), "PUT should appear in OpenAPI spec");
}

// --- State<T> tests ---

struct Config {
    app_name: String,
}

#[get("/state-info")]
async fn get_state_info(cfg: State<Config>) -> User {
    User { id: 0, name: cfg.app_name.clone(), email: "config@test.com".into() }
}

#[get("/state-user/{id}")]
async fn get_state_user(id: i64, db: Dep<Database>, cfg: State<Config>) -> Result<User, ApiError> {
    let mut user = db.get_user(id).await
        .ok_or_else(|| ApiError::not_found(format!("User {} not found", id)))?;
    user.name = format!("{} ({})", user.name, cfg.app_name);
    Ok(user)
}

async fn spawn_state_app() -> String {
    let app = HayaiApp::new()
        .dep(Database)
        .dep(Config { app_name: "TestApp".into() })
        .into_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_state_extractor_works() {
    let base = spawn_state_app().await;
    let resp = reqwest::get(format!("{base}/state-info")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "TestApp");
}

#[tokio::test]
async fn test_state_and_dep_together() {
    let base = spawn_state_app().await;
    let resp = reqwest::get(format!("{base}/state-user/42")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Alice (TestApp)");
}

#[tokio::test]
async fn test_state_missing_returns_500() {
    // App without Config registered
    let app = HayaiApp::new()
        .dep(Database)
        .include(
            HayaiRouter::new("")
                .route(__HAYAI_ROUTE_GET_STATE_INFO)
        )
        .into_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    let base = format!("http://{}", addr);
    let resp = reqwest::get(format!("{base}/state-info")).await.unwrap();
    assert_eq!(resp.status(), 500);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("State not registered"));
}

// ===== Cross-field Validation Tests =====

/// Custom validation function: end_date must be after start_date
fn validate_date_range(item: &DateRangeItem) -> Result<(), Vec<String>> {
    if item.end_date <= item.start_date {
        Err(vec!["end_date must be after start_date".to_string()])
    } else {
        Ok(())
    }
}

/// Use #[api_model(validate(custom = "fn_name"))] to enable cross-field validation
#[api_model(validate(custom = "validate_date_range"))]
#[derive(Debug, Clone)]
struct DateRangeItem {
    name: String,
    start_date: String,
    end_date: String,
}

#[post("/date-range")]
#[tag("validation")]
async fn create_date_range(body: DateRangeItem) -> DateRangeItem {
    body
}

async fn spawn_date_range_app() -> String {
    let app = HayaiApp::new()
        .title("Date Range Test API")
        .version("0.1.0")
        .include(
            HayaiRouter::new("")
                .route(__HAYAI_ROUTE_CREATE_DATE_RANGE)
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_cross_field_validation_valid_dates_returns_201() {
    let base = spawn_date_range_app().await;
    let client = reqwest::Client::new();
    let resp = client.post(format!("{base}/date-range"))
        .json(&serde_json::json!({
            "name": "Summer Vacation",
            "start_date": "2024-06-01",
            "end_date": "2024-08-31"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Summer Vacation");
}

#[tokio::test]
async fn test_cross_field_validation_invalid_dates_returns_422() {
    let base = spawn_date_range_app().await;
    let client = reqwest::Client::new();
    let resp = client.post(format!("{base}/date-range"))
        .json(&serde_json::json!({
            "name": "Invalid Range",
            "start_date": "2024-08-01",
            "end_date": "2024-06-01"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "Validation failed");
    let details = body["details"].as_array().unwrap();
    assert!(details.iter().any(|d| d.as_str().unwrap().contains("end_date must be after start_date")));
}

#[tokio::test]
async fn test_cross_field_validation_same_date_returns_422() {
    let base = spawn_date_range_app().await;
    let client = reqwest::Client::new();
    let resp = client.post(format!("{base}/date-range"))
        .json(&serde_json::json!({
            "name": "Same Day",
            "start_date": "2024-06-01",
            "end_date": "2024-06-01"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    let details = body["details"].as_array().unwrap();
    assert!(details.iter().any(|d| d.as_str().unwrap().contains("end_date must be after start_date")));
}
