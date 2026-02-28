//! SQLx Integration Tests for UltraAPI
//!
//! This test module verifies the SQLx ORM integration pattern works correctly
//! with UltraAPI's dependency injection system.

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use ultraapi::prelude::*;

// =============================================================================
// API Models
// =============================================================================

/// Item model for testing
#[api_model]
#[derive(Debug, Clone)]
struct Item {
    id: i64,
    name: String,
    description: String,
}

/// Create item request
#[api_model]
#[derive(Debug, Clone)]
struct CreateItemRequest {
    name: String,
    description: String,
}

// =============================================================================
// Database Initialization
// =============================================================================

/// Initialize the test database schema
async fn init_test_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT ''
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert sample data for testing
async fn insert_sample_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO items (name, description) VALUES ('Test Item 1', 'Description 1')")
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO items (name, description) VALUES ('Test Item 2', 'Description 2')")
        .execute(pool)
        .await?;

    sqlx::query("INSERT INTO items (name, description) VALUES ('Test Item 3', 'Description 3')")
        .execute(pool)
        .await?;

    Ok(())
}

// =============================================================================
// Route Handlers
// =============================================================================

/// Get all items from the database
#[get("/items")]
async fn get_items(pool: Dep<SqlitePool>) -> Result<Vec<Item>, ApiError> {
    let rows = sqlx::query("SELECT id, name, description FROM items ORDER BY id ASC")
        .fetch_all(&*pool)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let items: Vec<Item> = rows
        .into_iter()
        .map(|row| Item {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
        })
        .collect();

    Ok(items)
}

/// Get a single item by ID
#[get("/items/{id}")]
async fn get_item(id: i64, pool: Dep<SqlitePool>) -> Result<Item, ApiError> {
    let row = sqlx::query("SELECT id, name, description FROM items WHERE id = ?")
        .bind(id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Item with id {} not found", id)))?;

    Ok(Item {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
    })
}

/// Create a new item
#[post("/items")]
#[status(201)]
async fn create_item(body: CreateItemRequest, pool: Dep<SqlitePool>) -> Result<Item, ApiError> {
    let result = sqlx::query("INSERT INTO items (name, description) VALUES (?, ?)")
        .bind(&body.name)
        .bind(&body.description)
        .execute(&*pool)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let id = result.last_insert_rowid();

    Ok(Item {
        id,
        name: body.name,
        description: body.description,
    })
}

// =============================================================================
// Router
// =============================================================================

fn api_router() -> UltraApiRouter {
    UltraApiRouter::new("/")  // This should now work with double-slash normalization
        .route(__HAYAI_ROUTE_GET_ITEMS)
        .route(__HAYAI_ROUTE_GET_ITEM)
        .route(__HAYAI_ROUTE_CREATE_ITEM)
}

// =============================================================================
// Tests
// =============================================================================

/// Helper to create an in-memory SQLite pool for testing
async fn create_test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database pool");

    // Initialize schema
    init_test_db(&pool)
        .await
        .expect("Failed to initialize test database");

    // Insert sample data
    insert_sample_data(&pool)
        .await
        .expect("Failed to insert sample data");

    pool
}

/// Helper to create a test app with routes
fn create_test_app() -> UltraApiApp {
    UltraApiApp::new()
        .title("SQLx Test API")
        .version("0.1.0")
        .include(api_router())
}

#[tokio::test]
async fn test_sqlx_get_items_returns_all_items() {
    // Create test database pool
    let pool = create_test_pool().await;

    // Create the app with the pool as a dependency
    let app = create_test_app().dep(pool);

    // Use TestClient to make requests
    let client = TestClient::new(app).await;

    // Call GET /items
    let response = client.get("/items").await;
    assert_eq!(response.status(), 200, "Expected 200 OK");

    // Parse response as Vec<Item>
    let items: Vec<Item> = response.json().await.expect("Failed to parse response");

    // Verify we got all 3 items
    assert_eq!(items.len(), 3, "Expected 3 items");

    // Verify item contents
    assert_eq!(items[0].name, "Test Item 1");
    assert_eq!(items[0].description, "Description 1");
    assert_eq!(items[1].name, "Test Item 2");
    assert_eq!(items[2].name, "Test Item 3");
}

#[tokio::test]
async fn test_sqlx_get_item_by_id() {
    let pool = create_test_pool().await;

    let app = create_test_app().dep(pool);
    let client = TestClient::new(app).await;

    // Get item with id=2
    let response = client.get("/items/2").await;
    assert_eq!(response.status(), 200, "Expected 200 OK");

    let item: Item = response.json().await.expect("Failed to parse response");
    assert_eq!(item.id, 2);
    assert_eq!(item.name, "Test Item 2");
    assert_eq!(item.description, "Description 2");
}

#[tokio::test]
async fn test_sqlx_get_item_not_found() {
    let pool = create_test_pool().await;

    let app = create_test_app().dep(pool);
    let client = TestClient::new(app).await;

    // Try to get item that doesn't exist
    let response = client.get("/items/999").await;
    assert_eq!(response.status(), 404, "Expected 404 Not Found");
}

#[tokio::test]
async fn test_sqlx_create_item() {
    let pool = create_test_pool().await;

    let app = create_test_app().dep(pool);
    let client = TestClient::new(app).await;

    // Create a new item
    let new_item = CreateItemRequest {
        name: "New Item".to_string(),
        description: "New Description".to_string(),
    };

    let response = client.post("/items", &new_item).await;
    assert_eq!(response.status(), 201, "Expected 201 Created");

    let created: Item = response.json().await.expect("Failed to parse response");
    assert_eq!(created.id, 4, "New item should have id 4");
    assert_eq!(created.name, "New Item");
    assert_eq!(created.description, "New Description");

    // Verify it was actually inserted by fetching all items
    let response = client.get("/items").await;
    let items: Vec<Item> = response.json().await.expect("Failed to parse response");
    assert_eq!(items.len(), 4, "Should now have 4 items");
}

#[tokio::test]
async fn test_sqlx_pool_dependency_injection() {
    // This test verifies that the SqlitePool is properly injected
    // through UltraAPI's dependency injection system

    let pool = create_test_pool().await;

    // Create app with pool dependency
    let app = create_test_app().dep(pool);
    let client = TestClient::new(app).await;

    // The fact that this request succeeds proves DI is working
    let response = client.get("/items").await;
    assert!(response.status().is_success());
}
