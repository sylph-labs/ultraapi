//! SQLx Example for UltraAPI
//!
//! This example demonstrates how to integrate sqlx (async SQL) with UltraAPI
//! using SQLite as the database. It shows the recommended pattern for:
//! - Database connection pooling via dependency injection
//! - CRUD operations with UltraAPI routes
//! - In-memory SQLite for testing
//!
//! Run with:
//! ```sh
//! cd examples/sqlx-example
//! cargo run
//! ```
//!
//! Then test with:
//! ```sh
//! # Create an item
//! curl -X POST http://localhost:3000/items \
//!   -H "Content-Type: application/json" \
//!   -d '{"name": "Test Item", "description": "A test item"}'
//!
//! # Get all items
//! curl http://localhost:3000/items
//!
//! # Get a specific item
//! curl http://localhost:3000/items/1
//!
//! # Update an item
//! curl -X PUT http://localhost:3000/items/1 \
//!   -H "Content-Type: application/json" \
//!   -d '{"name": "Updated Item", "description": "Updated description", "completed": true}'
//!
//! # Delete an item
//! curl -X DELETE http://localhost:3000/items/1
//! ```

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use ultraapi::prelude::*;

// ============== API Models ==============

/// Item model for the API
#[api_model]
#[derive(Debug, Clone)]
struct Item {
    id: i64,
    name: String,
    description: String,
    completed: bool,
    created_at: String,
}

/// Create item request
#[api_model]
#[derive(Debug, Clone)]
struct CreateItem {
    #[validate(min_length = 1, max_length = 100)]
    name: String,

    #[validate(max_length = 500)]
    description: String,
}

/// Update item request
#[api_model]
#[derive(Debug, Clone)]
struct UpdateItem {
    #[validate(min_length = 1, max_length = 100)]
    name: String,

    #[validate(max_length = 500)]
    description: String,

    completed: bool,
}

// ============== Database Functions ==============

/// Initialize the database schema
async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            completed BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

// ============== Route Handlers ==============

/// Get all items
#[get("/items")]
async fn get_items(pool: Dep<SqlitePool>) -> Result<Vec<Item>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, name, description, completed, created_at FROM items ORDER BY id DESC",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let items: Vec<Item> = rows
        .into_iter()
        .map(|row| Item {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            completed: row.get("completed"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(items)
}

/// Get a specific item by ID
#[get("/items/{id}")]
async fn get_item(id: i64, pool: Dep<SqlitePool>) -> Result<Item, ApiError> {
    let row =
        sqlx::query("SELECT id, name, description, completed, created_at FROM items WHERE id = ?")
            .bind(id)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| ApiError::not_found(format!("Item with id {} not found", id)))?;

    Ok(Item {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        completed: row.get("completed"),
        created_at: row.get("created_at"),
    })
}

/// Create a new item
#[post("/items")]
async fn create_item(body: CreateItem, pool: Dep<SqlitePool>) -> Result<Item, ApiError> {
    let result =
        sqlx::query("INSERT INTO items (name, description, completed) VALUES (?, ?, false)")
            .bind(&body.name)
            .bind(&body.description)
            .execute(&*pool)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let id = result.last_insert_rowid();

    // Fetch the created item
    let row =
        sqlx::query("SELECT id, name, description, completed, created_at FROM items WHERE id = ?")
            .bind(id)
            .fetch_one(&*pool)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Item {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        completed: row.get("completed"),
        created_at: row.get("created_at"),
    })
}

/// Update an existing item
#[put("/items/{id}")]
async fn update_item(id: i64, body: UpdateItem, pool: Dep<SqlitePool>) -> Result<Item, ApiError> {
    // First check if item exists (we'll use this below)
    let _existing =
        sqlx::query("SELECT id, name, description, completed, created_at FROM items WHERE id = ?")
            .bind(id)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| ApiError::not_found(format!("Item with id {} not found", id)))?;

    // Build update query
    sqlx::query("UPDATE items SET name = ?, description = ?, completed = ? WHERE id = ?")
        .bind(&body.name)
        .bind(&body.description)
        .bind(body.completed)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Fetch the updated item
    let row =
        sqlx::query("SELECT id, name, description, completed, created_at FROM items WHERE id = ?")
            .bind(id)
            .fetch_one(&*pool)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Item {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        completed: row.get("completed"),
        created_at: row.get("created_at"),
    })
}

/// Delete an item
#[delete("/items/{id}")]
async fn delete_item(id: i64, pool: Dep<SqlitePool>) -> Result<(), ApiError> {
    let result = sqlx::query("DELETE FROM items WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(format!(
            "Item with id {} not found",
            id
        )));
    }

    Ok(())
}

// ============== App Router ==============

fn api_router() -> UltraApiRouter {
    UltraApiRouter::new("/")
        .tag("items")
        .route(__HAYAI_ROUTE_GET_ITEMS)
        .route(__HAYAI_ROUTE_GET_ITEM)
        .route(__HAYAI_ROUTE_CREATE_ITEM)
        .route(__HAYAI_ROUTE_UPDATE_ITEM)
        .route(__HAYAI_ROUTE_DELETE_ITEM)
}

// ============== Main ==============

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // For file-based SQLite:
    // let database_url = "sqlite:./items.db";

    // For in-memory SQLite (useful for development/testing):
    let database_url = "sqlite::memory:";

    println!("Connecting to database: {}", database_url);

    // Create the connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Initialize the schema
    init_db(&pool).await?;

    println!("Database initialized successfully");

    // Add some sample data
    let _ = sqlx::query("INSERT INTO items (name, description) VALUES ('Welcome', 'First item')")
        .execute(&pool)
        .await;

    println!("Starting SQLx Example server...");
    println!("API available at: http://localhost:3000");
    println!("OpenAPI docs at: http://localhost:3000/docs");

    // Create the UltraAPI app with the database pool as a dependency
    let app = UltraApiApp::new()
        .title("SQLx Example API")
        .version("0.1.0")
        .description("Example API demonstrating sqlx integration with UltraAPI")
        .dep(pool)
        .include(api_router());

    // Run the server
    app.serve("0.0.0.0:3000").await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create an in-memory database pool for testing
    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database pool");

        // Initialize schema
        init_db(&pool)
            .await
            .expect("Failed to initialize test database");

        pool
    }

    #[tokio::test]
    async fn test_crud_operations() {
        let pool = create_test_pool().await;

        // Create an item
        let result =
            sqlx::query("INSERT INTO items (name, description, completed) VALUES (?, ?, false)")
                .bind("Test Item")
                .bind("Test description")
                .execute(&pool)
                .await
                .expect("Failed to insert item");

        let id = result.last_insert_rowid();
        assert!(id > 0);

        // Read the item
        let row = sqlx::query("SELECT id, name, description, completed FROM items WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch item");

        assert_eq!(row.get::<String, _>("name"), "Test Item");
        assert_eq!(row.get::<String, _>("description"), "Test description");
        assert_eq!(row.get::<bool, _>("completed"), false);

        // Update the item
        sqlx::query("UPDATE items SET completed = true WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .expect("Failed to update item");

        let row = sqlx::query("SELECT completed FROM items WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch updated item");

        assert_eq!(row.get::<bool, _>("completed"), true);

        // Delete the item
        sqlx::query("DELETE FROM items WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .expect("Failed to delete item");

        let result = sqlx::query("SELECT id FROM items WHERE id = ?")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .expect("Failed to check deleted item");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_and_get_items() {
        let pool = create_test_pool().await;

        // Insert test data
        sqlx::query("INSERT INTO items (name, description) VALUES ('API Test', 'Testing')")
            .execute(&pool)
            .await
            .expect("Failed to insert test data");

        // Verify we can retrieve it
        let rows = sqlx::query("SELECT id, name, description, completed, created_at FROM items")
            .fetch_all(&pool)
            .await
            .expect("Failed to fetch items");

        assert_eq!(rows.len(), 1);

        let row = &rows[0];
        assert_eq!(row.get::<String, _>("name"), "API Test");
        assert_eq!(row.get::<String, _>("description"), "Testing");
    }
}
