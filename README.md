# UltraAPI

A FastAPI-inspired Rust web framework with automatic OpenAPI/Swagger documentation generation.

## Features

- **Automatic OpenAPI Generation**: Every route automatically gets documented in OpenAPI 3.1 format
- **Swagger UI**: Built-in `/docs` endpoint serves interactive API documentation
- **Type-Safe**: Full type inference with Rust's compile-time checks
- **Dependency Injection**: First-class support for `Dep<T>` and `State<T>` extractors
- **Validation**: Built-in validation with `#[validate]` attributes (email, min/max length, pattern, numeric ranges)
- **Router Composition**: Nested routers with prefix concatenation and tag/security propagation
- **Result Handler**: Automatic `Result<T, ApiError>` handling with proper HTTP status codes
- **Bearer Auth**: Easy JWT bearer authentication setup

## Quick Start

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Serialize, Deserialize, JsonSchema)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[post("/users")]
async fn create_user(user: User) -> User {
    User {
        id: 1,
        name: user.name,
        email: user.email,
    }
}

#[get("/users/{id}")]
async fn get_user(id: i64) -> Result<User, ApiError> {
    // Return Result<T, ApiError> for automatic error handling
    Ok(User {
        id,
        name: "Alice".into(),
        email: "alice@example.com".into(),
    })
}

#[derive(Clone)]
struct Database;

impl Database {
    async fn get_user(&self, id: i64) -> Option<User> {
        Some(User { id, name: "Alice".into(), email: "alice@example.com".into() })
    }
}

#[tokio::main]
async fn main() {
    let app = UltraApiApp::new()
        .title("My API")
        .version("1.0.0")
        .dep(Database)
        .route(create_user)
        .route(get_user);

    app.serve("0.0.0.0:3000").await;
}
```

## API Reference

### Macros

- `#[get(path)]` - Register a GET endpoint
- `#[post(path)]` - Register a POST endpoint  
- `#[put(path)]` - Register a PUT endpoint
- `#[delete(path)]` - Register a DELETE endpoint
- `#[api_model]` - Generate validation and OpenAPI schema for a struct/enum
- `#[status(N)]` - Set custom HTTP status code for a route
- `#[tag("name")]` - Add tags for OpenAPI grouping
- `#[security("scheme")]` - Apply security scheme to a route

### Extractors

- `Dep<T>` - Inject dependencies registered with `.dep()`
- `State<T>` - Inject app state with type safety

### Validation Attributes

- `#[validate(email)]` - Validate as email address
- `#[validate(min_length = N)]` - Minimum string length
- `#[validate(max_length = N)]` - Maximum string length
- `#[validate(minimum = N)]` - Minimum numeric value
- `#[validate(maximum = N)]` - Maximum numeric value
- `#[validate(pattern = "regex")]` - Pattern match
- `#[validate(min_items = N)]` - Minimum array length

## OpenAPI Endpoints

- `GET /openapi.json` - Raw OpenAPI 3.1 spec
- `GET /docs` - Swagger UI

## License

MIT
