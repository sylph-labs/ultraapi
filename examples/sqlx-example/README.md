# SQLx Example

This example demonstrates how to integrate **sqlx** (async SQL database library) with **UltraAPI** using SQLite.

## Overview

The example shows:
- Database connection pooling via UltraAPI's dependency injection (`Dep<SqlitePool>`)
- CRUD operations (Create, Read, Update, Delete) with UltraAPI routes
- In-memory SQLite for easy testing
- Full integration with UltraAPI's OpenAPI generation

## Requirements

```toml
[dependencies]
ultraapi = { version = "0.1", features = ["sqlite"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
```

Note: Enable the `sqlite` feature in UltraAPI.

## Database Setup

### Option 1: In-Memory SQLite (Development/Testing)

For development and testing, use in-memory SQLite:

```rust
let database_url = "sqlite::memory:";
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect(database_url)
    .await?;
```

### Option 2: File-Based SQLite (Production)

For persistent storage, use a file:

```rust
let database_url = "sqlite:./items.db";
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect(database_url)
    .await?;
```

### Option 3: PostgreSQL or MySQL

Sqlx supports PostgreSQL and MySQL as well:

```rust
// PostgreSQL
let database_url = "postgres://user:password@localhost/mydb";

// MySQL  
let database_url = "mysql://user:password@localhost/mydb";

let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect(&database_url)
    .await?;
```

## Migrations

### Using sqlx-cli

For production databases, use migrations:

```bash
# Install sqlx-cli
cargo install sqlx-cli

# Create a new migration
sqlx migrate add -r init

# Run migrations
sqlx database setup --database-url sqlite:./mydb.db
```

### Programmatic Migrations

Alternatively, run migrations programmatically at startup:

```rust
async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            completed BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;
    
    Ok(())
}
```

## Dependency Injection Pattern

Register the database pool as a dependency:

```rust
let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .dep(pool)  // Register SqlitePool
    .include(api_router());
```

Then use it in route handlers:

```rust
#[get("/items")]
async fn get_items(pool: Dep<SqlitePool>) -> Result<Vec<Item>, ApiError> {
    let rows = sqlx::query!("SELECT * FROM items")
        .fetch_all(&*pool)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;
    
    // ... convert rows to items
    Ok(items)
}
```

## Running the Example

```bash
cd examples/sqlx-example
cargo run
```

Then test the API:

```bash
# Create an item
curl -X POST http://localhost:3000/items \
  -H "Content-Type: application/json" \
  -d '{"name": "Test Item", "description": "A test item"}'

# Get all items
curl http://localhost:3000/items

# Get a specific item
curl http://localhost:3000/items/1

# Update an item
curl -X PUT http://localhost:3000/items/1 \
  -H "Content-Type: application/json" \
  -d '{"name": "Updated Item", "completed": true}'

# Delete an item
curl -X DELETE http://localhost:3000/items/1
```

## Testing

Run tests with in-memory database:

```bash
cargo test -p sqlx-example
```

The example includes integration tests that:
1. Create an in-memory SQLite database
2. Test CRUD operations directly via sqlx
3. Test the full API using UltraAPI's TestClient

## Combining with GraphQL

For GraphQL + SQLx integration, combine this example with `examples/graphql-example`:

1. Register `SqlitePool` as a dependency
2. Create an async-graphql schema that uses the pool
3. Use `route_axum` to add GraphQL endpoints

See [docs/graphql.md](../docs/graphql.md) for GraphQL details.
