# UltraAPI

[![crates.io](https://img.shields.io/crates/v/ultraapi.svg)](https://crates.io/crates/ultraapi)
[![docs.rs](https://docs.rs/ultraapi/badge.svg)](https://docs.rs/ultraapi)
[![CI](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml/badge.svg)](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml)

> **日本語**: [日本語版README (README.ja.md)](./README.ja.md) もございます。

A FastAPI-inspired Rust web framework with automatic OpenAPI/Swagger documentation generation.

## Features

- **Automatic OpenAPI Generation**: Every route automatically gets documented in OpenAPI 3.1 format
- **Swagger UI**: Built-in `/docs` endpoint serves interactive API documentation
- **Type-Safe**: Full type inference with Rust's compile-time checks
- **Dependency Injection**: First-class support for `Dep<T>`, `State<T>`, and `Depends<T>` extractors
- **Yield Dependencies**: FastAPI-style generator dependencies with cleanup hooks and scope management (function/request)
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
- `#[response_class("json"|"html"|"text"|"binary"|"stream"|"xml")]` - Set response content type

### Response Model Shaping

UltraAPI supports FastAPI-like response model shaping with `include`, `exclude`, and `by_alias` options:

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct UserProfile {
    id: i64,
    username: String,
    email: String,
    password_hash: String,
    created_at: String,
    is_admin: bool,
}

// Include only specific fields in the response
#[get("/users/{id}/public", response_model(include={"id", "username"}))]
async fn get_public_profile(id: i64) -> UserProfile {
    UserProfile {
        id,
        username: "alice".into(),
        email: "alice@example.com".into(),
        password_hash: "secret".into(),
        created_at: "2024-01-01".into(),
        is_admin: false,
    }
}

// Exclude sensitive fields from the response
#[get("/users/{id}/profile", response_model(exclude={"password_hash"}))]
async fn get_user_profile(id: i64) -> UserProfile {
    UserProfile {
        id,
        username: "alice".into(),
        email: "alice@example.com".into(),
        password_hash: "secret".into(),
        created_at: "2024-01-01".into(),
        is_admin: false,
    }
}

// Use alias names (from serde(rename)) for serialization
#[get("/users/{id}/api", response_model(by_alias=true))]
async fn get_user_api(id: i64) -> UserProfile {
    UserProfile {
        id,
        username: "alice".into(),
        email: "alice@example.com".into(),
        password_hash: "secret".into(),
        created_at: "2024-01-01".into(),
        is_admin: false,
    }
}

// Combine include and exclude (include takes precedence)
#[get("/users/{id}/summary", response_model(include={"id", "username"}, exclude={"email"}))]
async fn get_user_summary(id: i64) -> UserProfile {
    UserProfile {
        id,
        username: "alice".into(),
        email: "alice@example.com".into(),
        password_hash: "secret".into(),
        created_at: "2024-01-01".into(),
        is_admin: false,
    }
}
```

### Field-Level Attributes for api_model

UltraAPI supports custom field attributes for controlling serialization behavior:

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone)]
struct UserResponse {
    // Field alias - serializes with the alias name
    #[alias("userId")]
    user_id: i64,
    
    // Skip serialization (field not included in JSON output)
    #[skip_serializing]
    internal_note: String,
    
    // Skip deserialization (field uses default value when parsing JSON)
    #[skip_deserializing]
    computed_field: String,
    
    // Skip both serialization and deserialization
    #[skip]
    private_data: String,
}

// Standard serde attributes are also supported:
#[api_model]
#[derive(Debug, Clone)]
struct LegacyResponse {
    #[serde(rename = "userId")]
    user_id: i64,
    
    #[serde(skip_serializing)]
    internal: String,
    
    #[serde(skip)]
    hidden: String,
}
```

**Note:** When using `#[skip_deserializing]`, the field will receive its type's default value (e.g., empty `String`, `0` for integers) when deserializing, regardless of any value present in the JSON input.

#### Input/Output Schema Separation (FastAPI-style)

UltraAPI supports FastAPI-like `read_only` and `write_only` field attributes for automatic input/output schema separation:

```rust
use ultraapi::prelude::*;

#[api_model]
struct User {
    /// User ID (only in response - read only)
    #[read_only]
    id: i64,
    
    /// Username (in both request and response)
    username: String,
    
    /// Password (only in request - write only)
    #[write_only]
    password: String,
    
    /// Email (in both request and response)
    email: String,
}
```

**Behavior:**
- `#[read_only]`: Field appears in responses but NOT in request bodies (sets `readOnly: true` in OpenAPI, adds `skip_deserializing` to serde)
- `#[write_only]`: Field appears in requests but NOT in responses (sets `writeOnly: true` in OpenAPI, adds `skip_serializing` to serde)

This is useful for:
- Password fields that should be accepted in create requests but never returned
- Auto-generated IDs that are returned but never accepted as input
- Internal timestamps or computed fields

**Caveats:**
- The `by_alias=true` option in `response_model` works with both `#[alias(...)]` and `#[serde(rename = "...")]` attributes
- Fields marked with `#[skip_serializing]` are still included in the OpenAPI schema (since they can still be deserialized)
- For complete control over schema generation, use response_model `include`/`exclude` options at the route level

Note: The include/exclude filtering works recursively on nested objects and arrays. When both `include` and `exclude` are specified, `include` takes precedence.

### Response Class

UltraAPI supports specifying different response content types using the `response_class` attribute. This controls both the runtime response Content-Type header and the OpenAPI specification:

```rust
use ultraapi::prelude::*;

// Default JSON response (implicit)
#[get("/users/{id}")]
async fn get_user(id: i64) -> User {
    User { id, name: "Alice".into() }
}

// Explicit JSON response
#[get("/users/{id}/json", response_class("json"))]
async fn get_user_json(id: i64) -> User {
    User { id, name: "Alice".into() }
}

// HTML response
#[get("/html")]
#[response_class("html")]
async fn get_html() -> String {
    "<html><body><h1>Hello</h1></body></html>".to_string()
}

// Plain text response
#[get("/text")]
#[response_class("text")]
async fn get_text() -> String {
    "Plain text content".to_string()
}

// Binary/octet-stream response
#[get("/download")]
#[response_class("binary")]
async fn download_file() -> Vec<u8> {
    vec![0x00, 0x01, 0x02, 0xFF]
}

// Streaming response (also application/octet-stream)
#[get("/stream")]
#[response_class("stream")]
async fn stream_data() -> String {
    "Streaming content".to_string()
}

// XML response
#[get("/data.xml")]
#[response_class("xml")]
async fn get_xml() -> String {
    "<data><item>value</item></data>".to_string()
}
```

Valid `response_class` values:
- `"json"` - Default, returns `application/json`
- `"html"` - Returns `text/html`
- `"text"` - Returns `text/plain`
- `"binary"` - Returns `application/octet-stream`
- `"stream"` - Returns `application/octet-stream` (for streaming responses)
- `"xml"` - Returns `application/xml`

The OpenAPI specification will automatically reflect the correct content-type for each endpoint.

### Security Schemes

UltraAPI supports multiple security schemes for OpenAPI documentation:

```rust
use ultraapi::prelude::*;

// Bearer Authentication (JWT)
let app = UltraApiApp::new()
    .bearer_auth();

// API Key Authentication
let app = UltraApiApp::new()
    .api_key("apiKeyAuth", "X-API-Key", "header");

// OAuth2 - Implicit Flow
let app = UltraApiApp::new()
    .oauth2_implicit(
        "oauth2Implicit",
        "https://example.com/authorize",
        [("read", "Read access"), ("write", "Write access")],
    );

// OAuth2 - Password Flow
let app = UltraApiApp::new()
    .oauth2_password(
        "oauth2Password",
        "https://example.com/token",
        [("read", "Read access"), ("write", "Write access")],
    );

// OAuth2 - Client Credentials Flow
let app = UltraApiApp::new()
    .oauth2_client_credentials(
        "oauth2ClientCredentials",
        "https://example.com/token",
        [("read", "Read access")],
    );

// OAuth2 - Authorization Code Flow
let app = UltraApiApp::new()
    .oauth2_authorization_code(
        "oauth2AuthCode",
        "https://example.com/authorize",
        "https://example.com/token",
        [("read", "Read access"), ("write", "Write access")],
    );

// OpenID Connect
let app = UltraApiApp::new()
    .openid_connect("oidc", "https://example.com/.well-known/openid-configuration");

// Protect routes with #[security("schemeName")]
#[get("/protected")]
#[security("oauth2AuthCode")]
async fn protected_route() -> String {
    "secret data".to_string()
}
```

### Runtime Auth Enforcement

UltraAPI supports runtime enforcement of security requirements via middleware:

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::{SecuritySchemeConfig, ScopedAuthValidator, MockAuthValidator};

// Enable auth middleware with default mock validator
let app = UltraApiApp::new()
    .title("Secure API")
    .version("1.0.0")
    .bearer_auth()
    .middleware(|builder| {
        builder.enable_auth()  // Enforce #[security] routes at runtime
    });

// With custom API keys
let app = UltraApiApp::new()
    .api_key("apiKeyAuth", "X-API-Key", "header")
    .middleware(|builder| {
        builder.enable_auth_with_api_keys(vec!["my-secret-key".to_string()])
    });

// API Key in query parameter
let app = UltraApiApp::new()
    .security_scheme(
        "apiKeyAuth",
        ultraapi::openapi::SecurityScheme::ApiKey {
            name: "api_key".to_string(),
            location: "query".to_string(),
        },
    )
    .middleware(|builder| {
        builder
            .enable_auth_with_api_keys(vec!["valid-key".to_string()])
            .with_security_scheme(
                SecuritySchemeConfig::api_key_query("apiKeyAuth", "api_key")
            )
    });

// API Key in cookie
let app = UltraApiApp::new()
    .security_scheme(
        "apiKeyAuth",
        ultraapi::openapi::SecurityScheme::ApiKey {
            name: "session".to_string(),
            location: "cookie".to_string(),
        },
    )
    .middleware(|builder| {
        builder
            .enable_auth_with_api_keys(vec!["session-key".to_string()])
            .with_security_scheme(
                SecuritySchemeConfig::api_key_cookie("apiKeyAuth", "session")
            )
    });

// With scope-based authorization
let validator = ScopedAuthValidator::new(MockAuthValidator::new())
    .with_scope("admin-token", vec!["read".to_string(), "write".to_string(), "admin".to_string()]);

let app = UltraApiApp::new()
    .bearer_auth()
    .middleware(|builder| {
        builder
            .enable_auth_with_validator(validator)
            .with_security_scheme(
                SecuritySchemeConfig::bearer("bearerAuth")
                    .with_scopes(vec!["admin".to_string()])
            )
    });

// Protect routes with scopes
#[get("/admin-only")]
#[security("bearerAuth")]
async fn admin_route() -> String {
    "admin data".to_string()
}
```

### Extractors

- `Dep<T>` - Inject dependencies registered with `.dep()`
- `State<T>` - Inject app state with type safety
- `Depends<T>` - FastAPI-style dependency injection with nested support

### Yield Dependencies (FastAPI-style)

UltraAPI supports generator-based dependencies with cleanup hooks, similar to FastAPI's `yield` dependencies:

```rust
use ultraapi::prelude::*;
use std::sync::Arc;

// Define a resource with cleanup
struct DatabasePool { connection_string: String }

#[async_trait::async_trait]
impl Generator for DatabasePool {
    type Output = Self;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        // Setup: connect to database
        Ok(Arc::try_unwrap(self).unwrap_or_else(|a| (*a).clone()))
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        // Cleanup: close connection
        println!("Closing database connection");
        Ok(())
    }
}

// Register with function scope (cleanup runs before response)
let app = UltraApiApp::new()
    .yield_depends(Arc::new(DatabasePool { connection_string: "...".into() }), Scope::Function);

// Or with request scope (cleanup runs after response)
let app = UltraApiApp::new()
    .yield_depends(Arc::new(DatabasePool { connection_string: "...".into() }), Scope::Request);
```

- **Function scope**: Cleanup runs before the handler returns its response
- **Request scope**: Cleanup runs after the entire request handling completes, and each request gets a fresh dependency instance

### Validation Attributes

- `#[validate(email)]` - Validate as email address
- `#[validate(min_length = N)]` - Minimum string length
- `#[validate(max_length = N)]` - Maximum string length
- `#[validate(minimum = N)]` - Minimum numeric value
- `#[validate(maximum = N)]` - Maximum numeric value
- `#[validate(pattern = "regex")]` - Pattern match
- `#[validate(min_items = N)]` - Minimum array length

### Input/Output Schema Attributes

- `#[read_only]` - Field appears only in responses (not in request bodies)  
  (Derived from `#[serde(skip_deserializing)]`)
- `#[write_only]` - Field appears only in requests (not in responses)
  (Derived from `#[serde(skip_serializing)]`)

> Note: OpenAPI `readOnly` and `writeOnly` properties are automatically extracted from schemars metadata.

### OpenAPI Endpoints

- `GET /openapi.json` - Raw OpenAPI 3.1 spec
- `GET /docs` - Swagger UI

## Testing

### OpenAPI FastAPI Parity Golden Tests

UltraAPI includes golden tests to ensure OpenAPI output parity with FastAPI. These tests compare generated OpenAPI schemas against a known-good snapshot to catch regressions.

**Test file:** `ultraapi/tests/openapi_fastapi_parity_tests.rs`

**Golden file:** `ultraapi/tests/golden/openapi_fastapi_parity.json`

The test validates:
- Path operations (GET, POST, PUT, DELETE)
- Path parameters with proper `in: path` specification
- Query parameters from struct extractors
- Request body schemas with validation constraints (`minLength`, `maxLength`, `minimum`, `pattern`)
- Response schemas with `$ref` to components/schemas
- Components/schemas with proper type definitions

**To update the golden file** (after intentional OpenAPI output changes):

```bash
cd ultraapi
UPDATE_GOLDEN=1 cargo test test_openapi_fastapi_parity_regenerate
```

This will regenerate the golden snapshot at `tests/golden/openapi_fastapi_parity.json`. Review the diff to ensure changes are intentional, then commit the updated golden file.

## License

MIT
