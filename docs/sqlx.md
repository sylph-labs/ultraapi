# SQLx Integration (ORM)

UltraAPI integrates seamlessly with **sqlx**, the popular async SQL database library for Rust. This guide covers the recommended pattern for database integration.

## Overview

- **Database**: sqlx (supports SQLite, PostgreSQL, MySQL)
- **Pattern**: Dependency injection via `Dep<SqlitePool>`
- **Feature**: None required in UltraAPI (sqlx is used directly)

## Installation

```toml
[dependencies]
ultraapi = "0.1"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

Choose your database features:
- `sqlite` - For SQLite (file or in-memory)
- `postgres` - For PostgreSQL  
- `mysql` - For MySQL
- `any` - For runtime database selection

## Database Connection Pool

Create a connection pool and register it as a dependency:

```rust
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use ultraapi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SQLite: file-based
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:./mydb.db")
        .await?;

    // Or SQLite: in-memory (for testing)
    // let pool = SqlitePoolOptions::new()
    //     .max_connections(5)
    //     .connect("sqlite::memory:")
    //     .await?;

    let app = UltraApiApp::new()
        .title("My API")
        .version("1.0.0")
        .dep(pool)  // Register the pool
        .include(api_router());

    app.serve("0.0.0.0:3000").await;

    Ok(())
}
```

## Using the Pool in Routes

Inject the pool using `Dep<T>`:

```rust
use sqlx::SqlitePool;

#[get("/items")]
async fn get_items(pool: Dep<SqlitePool>) -> Result<Vec<Item>, ApiError> {
    let rows = sqlx::query!("SELECT id, name FROM items")
        .fetch_all(&*pool)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    // Convert to your API model
    let items: Vec<Item> = rows.into_iter().map(|r| Item {
        id: r.id,
        name: r.name,
        // ...
    }).collect();

    Ok(items)
}

#[post("/items")]
async fn create_item(
    body: CreateItemRequest,
    pool: Dep<SqlitePool>
) -> Result<Item, ApiError> {
    let result = sqlx::query(
        "INSERT INTO items (name, description) VALUES (?, ?)"
    )
    .bind(&body.name)
    .bind(&body.description)
    .execute(&*pool)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    let id = result.last_insert_rowid();

    // Fetch and return the created item
    let row = sqlx::query!("SELECT * FROM items WHERE id = ?", id)
        .fetch_one(&*pool)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    Ok(Item { /* ... */ })
}
```

## Migrations

### Option 1: sqlx-cli

```bash
# Install
cargo install sqlx-cli

# Create migration
sqlx migrate add -r init

# Run against database
sqlx database setup --database-url sqlite:./mydb.db
```

### Option 2: Programmatic

```rust
async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            completed BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Call in main or startup hook
init_db(&pool).await?;
```

## Testing

Use in-memory SQLite for tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");
            
        init_db(&pool).await.expect("Failed to init test DB");
        pool
    }

    #[tokio::test]
    async fn test_get_items() {
        let pool = test_pool().await;
        
        // Insert test data
        sqlx::query("INSERT INTO items (name) VALUES ('test')")
            .execute(&pool)
            .await
            .unwrap();

        // Test via UltraAPI
        let router = UltraApiRouter::new("").route(__HAYAI_ROUTE_GET_ITEMS);
        let app = UltraApiApp::new()
            .dep(pool)
            .include(router);
            
        let client = TestClient::new(app).await;
        let response = client.get("/items").await;
        
        assert!(response.status().is_success());
    }
}
```

Run tests:
```bash
cargo test -p sqlx-example
```

## Authentication

Combine with UltraAPI's built-in authentication:

```rust
#[get("/protected-items")]
#[security("bearer")]
async fn get_protected_items(
    token: OAuth2PasswordBearer,
    pool: Dep<SqlitePool>
) -> Result<Vec<Item>, ApiError> {
    // Verify token, then fetch items
    // ...
}
```

See [jwt.md](./jwt.md) for authentication details.

## Example

A complete runnable example is available:

- `examples/sqlx-example`

Run it:
```bash
cd examples/sqlx-example
cargo run
```

Then test:
```bash
curl http://localhost:3000/items
curl -X POST http://localhost:3000/items \
  -H "Content-Type: application/json" \
  -d '{"name": "New Item", "description": "Description"}'
```

## Combining with GraphQL

For GraphQL + sqlx integration, combine this pattern with `examples/graphql-example`:

1. Register `SqlitePool` as a dependency
2. Build an async-graphql schema that queries the database
3. Add GraphQL routes via `route_axum`

See [graphql.md](./graphql.md) for GraphQL setup.
