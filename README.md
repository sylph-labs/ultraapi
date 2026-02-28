# UltraAPI

[![crates.io](https://img.shields.io/crates/v/ultraapi.svg)](https://crates.io/crates/ultraapi)
[![docs.rs](https://docs.rs/ultraapi/badge.svg)](https://docs.rs/ultraapi)
[![CI](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml/badge.svg)](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml)

> **日本語**: [日本語版 README (README.ja.md)](./README.ja.md)

A FastAPI-inspired Rust web framework with automatic OpenAPI/Swagger documentation generation.

- **Concept**: **Rust Performance × FastAPI DX**
- **OpenAPI**: `GET /openapi.json`
- **Docs UI**: `GET /docs` (default: Embedded Scalar)
- **ReDoc UI**: `GET /redoc`

## Features

- **FastAPI-style route definitions**: `#[get]`, `#[post]`, `#[put]`, `#[delete]`
- **Automatic OpenAPI generation from serde/schemars**: Generate schemas from `#[api_model]` type definitions
- **Built-in /docs**: Provides API reference UI out of the box (CDN Swagger UI also available)
- **Automatic validation**: Returns 422 (Unprocessable Entity) with `#[validate(...)]`
- **DI (Dependency Injection)**: `Dep<T>`, `State<T>`, `Depends<T>`
- **Router composition**: Compose prefix / tags / security per router
- **WebSocket / SSE**: `#[ws]`, `#[sse]`
- **Lifespan hooks**: startup/shutdown

## Lifespan (Startup/Shutdown Hooks)

UltraAPI supports hooks that run at application startup and shutdown.

### Basic Usage

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new()
    .lifecycle(|lifecycle| {
        lifecycle
            .on_startup(|state| {
                Box::pin(async move {
                    println!("Starting application...");
                    // Establish database connections
                    // Load templates
                })
            })
            .on_shutdown(|state| {
                Box::pin(async move {
                    println!("Shutting down application...");
                    // Close database connections
                    // Clean up resources
                })
            })
    });
```

### Three Usage Patterns

#### 1. Using `serve()` (Recommended)

When using the `serve()` method, the startup hook runs when the server starts, and the shutdown hook runs during graceful shutdown (Ctrl+C).

```rust
#[tokio::main]
async fn main() {
    UltraApiApp::new()
        .lifecycle(|l| l
            .on_startup(|_| Box::pin(async { println!(" startup!"); }))
            .on_shutdown(|_| Box::pin(async { println!(" shutdown!"); }))
        )
        .serve("0.0.0.0:3000")
        .await;
}
```

#### 2. Using `TestClient` (For Testing)

In tests, `TestClient` automatically manages the lifecycle. Startup runs on the first request, and shutdown runs when the test ends (on Drop).

```rust
#[tokio::test]
async fn test_my_api() {
    let app = UltraApiApp::new()
        .lifecycle(|l| l
            .on_startup(|_| Box::pin(async { /* Test resources */ }))
            .on_shutdown(|_| Box::pin(async { /* Cleanup */ }))
        );
    
    let client = TestClient::new(app).await;
    
    // Execute request (startup runs at this point)
    let response = client.get("/api/items").await;
    
    // Shutdown is automatically called when test ends
    // Or you can call it explicitly
    client.shutdown().await;
}
```

#### 3. Using `into_router_with_lifespan()`

When using the router directly (for custom servers or other purposes), you can integrate the lifecycle using `into_router_with_lifespan()`.

```rust
let app = UltraApiApp::new()
    .lifecycle(|l| l
        .on_startup(|_| Box::pin(async { /* Startup logic */ }))
        .on_shutdown(|_| Box::pin(async { /* Shutdown logic */ }))
    );

let (router, runner) = app.into_router_with_lifespan();

// Start server using router...
// Example: axum::serve(listener, router).await

// Manually trigger shutdown on exit
runner.shutdown().await;
```

### Notes

- **Preventing multiple executions**: The startup hook runs only once on the first request. Internal locking prevents duplicate execution.
- **Using with `into_router()`**: The regular `into_router()` method does not integrate lifecycle. Use `into_router_with_lifespan()` when you need lifecycle support.
- **Lazy startup**: With `into_router_with_lifespan()` and `TestClient`, startup runs on the first request (lazy startup).

## Installation

```toml
[dependencies]
ultraapi = "0.1"
```

## Quick Start

### 1) Model Definition (OpenAPI + Validation)

```rust
use ultraapi::prelude::*;

/// User creation request
#[api_model]
#[derive(Debug, Clone)]
struct CreateUser {
    #[validate(min_length = 1, max_length = 100)]
    name: String,

    #[validate(email)]
    email: String,
}

/// User
#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
}
```

### 2) Route Definition (FastAPI-style)

```rust
use ultraapi::prelude::*;

#[post("/users")]
async fn create_user(body: CreateUser) -> User {
    User { id: 1, name: body.name, email: body.email }
}

#[get("/users/{id}")]
async fn get_user(id: i64) -> Result<User, ApiError> {
    Ok(User { id, name: "Alice".into(), email: "alice@example.com".into() })
}
```

### 3) Router Composition + Startup

Macros like `#[get]` automatically generate route refs (`__HAYAI_ROUTE_<FN>`).

```rust
use ultraapi::prelude::*;

fn api() -> UltraApiRouter {
    UltraApiRouter::new("/api")
        .tag("users")
        .route(__HAYAI_ROUTE_CREATE_USER)
        .route(__HAYAI_ROUTE_GET_USER)
}

#[tokio::main]
async fn main() {
    UltraApiApp::new()
        .title("My API")
        .version("1.0.0")
        .include(api())
        .serve("0.0.0.0:3000")
        .await;
}
```

After startup:

- OpenAPI: `GET /openapi.json`
- Docs: `GET /docs`

## CLI (ultraapi command)

UltraAPI includes a CLI tool (`ultraapi` command).

### Installation

```bash
cargo install ultraapi-cli
```

Or run directly from the ultraapi workspace:

```bash
cargo run --bin ultraapi -- --help
```

### Commands

#### Running Applications

```bash
# Run with default settings (0.0.0.0:3000)
ultraapi run ultraapi-example

# Specify host and port
ultraapi run ultraapi-example --host 127.0.0.1 --port 8080

# Enable verbose output
ultraapi -v run ultraapi-example --port 4000
```

#### Development Mode

```bash
# Run in development mode (currently same as run)
ultraapi dev ultraapi-example --host 0.0.0.0 --port 3001
```

> **Note**: Auto-reload feature is not yet implemented.

### Usage Examples

```bash
# Start examples/ultraapi-example on port 3001
cargo run --bin ultraapi -- run ultraapi-example --port 3001

# Start in development mode
cargo run --bin ultraapi -- dev ultraapi-example --port 3001
```

## Major Macros

- Routes: `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`, `#[head]`, `#[options]`, `#[trace]`
- Models: `#[api_model]`
- WebSocket: `#[ws]`
- SSE: `#[sse]`

### Additional Attributes for Routes

- `#[status(200)]` etc.: Success status code
- `#[tag("name")]`: OpenAPI tag
- `#[security("bearer")]`: Security requirement (reflected in OpenAPI and auth middleware)
- `#[security("basicAuth")]`: Basic auth (reflected in OpenAPI and auth middleware)
- `#[security("oauth2Password")]`: OAuth2 Password Flow (reflected in OpenAPI)
- `#[security("oauth2AuthCode")]`: OAuth2 Authorization Code Flow (reflected in OpenAPI)
- `#[security("oauth2Implicit")]`: OAuth2 Implicit Flow (reflected in OpenAPI)

#### OAuth2 Dependency Objects

UltraAPI provides FastAPI-compatible OAuth2 dependency objects:

```rust
use ultraapi::prelude::*;

/// OAuth2PasswordBearer: auto_error=true (default)
/// Returns 401 error when token is missing
#[get("/protected")]
async fn protected_endpoint(token: OAuth2PasswordBearer) -> String {
    format!("Token: {}", token.0)
}

/// OptionalOAuth2PasswordBearer: auto_error=false
/// Returns None instead of error when token is missing
#[get("/optional-protected")]
async fn optional_protected_endpoint(token: OptionalOAuth2PasswordBearer) -> String {
    match token.0 {
        Some(t) => format!("Token: {}", t),
        None => "No token provided".to_string(),
    }
}

/// OAuth2AuthorizationCodeBearer: For Authorization Code Flow
#[get("/auth-code-protected")]
async fn auth_code_protected_endpoint(token: OAuth2AuthorizationCodeBearer) -> String {
    format!("Auth Code Token: {}", token.0)
}
```

To use these dependency objects, you need to register the security scheme with the app:

```rust
let app = UltraApiApp::new()
    .title("OAuth2 API")
    .version("0.1.0")
    .oauth2_password(
        "oauth2Password",
        "https://example.com/token",
        [("read", "Read access"), ("write", "Write access")],
    )
    // Or
    .oauth2_authorization_code(
        "oauth2AuthCode",
        "https://example.com/authorize",
        "https://example.com/token",
        [("read", "Read access")],
    );
```

- `OAuth2PasswordBearer` / `OptionalOAuth2PasswordBearer`: For OAuth2 Password Flow
- `OAuth2AuthorizationCodeBearer` / `OptionalOAuth2AuthorizationCodeBearer`: For OAuth2 Authorization Code Flow
- `auto_error=true` (default): Returns 401 when token is missing
- `auto_error=false` (Optional* versions): Returns 200 with None when token is missing

#### OAuth2 Production Components

UltraAPI provides types and helpers needed for OAuth2 production use. These are accessible from `ultraapi::oauth2` or `ultraapi::prelude`.

For a complete guide on JWT authentication with AuthLayer validator integration, see [`docs/jwt.md`](docs/jwt.md).

```rust
use ultraapi::oauth2::{
    OAuth2PasswordRequestForm,
    TokenResponse,
    OAuth2ErrorResponse,
    OpaqueTokenValidator,
};
```

##### Example /token Endpoint Implementation

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::create_bearer_auth_error;

#[post("/token")]
async fn token(
    Form(form): Form<OAuth2PasswordRequestForm>,
) -> Result<Json<TokenResponse>, Json<OAuth2ErrorResponse>> {
    // Only support password grant
    if !form.is_password_grant() {
        return Err(Json(OAuth2ErrorResponse::unsupported_grant_type()));
    }
    
    // User authentication (verify with database in practice)
    let valid = verify_credentials(&form.username, &form.password);
    if !valid {
        return Err(Json(OAuth2ErrorResponse::invalid_grant(
            "Invalid username or password"
        )));
    }
    
    // Generate token
    let access_token = generate_token(&form.username, form.scopes());
    let response = TokenResponse::with_scopes(access_token, 3600, form.scopes());
    
    Ok(Json(response))
}
```

##### Custom Validator Implementation

Use the `OAuth2TokenValidator` trait to implement your own token validator:

```rust
use ultraapi::middleware::{OAuth2TokenValidator, TokenData, OAuth2AuthError};

struct MyTokenValidator;

#[async_trait::async_trait]
impl OAuth2TokenValidator for MyTokenValidator {
    async fn validate(&self, token: &str) -> Result<TokenData, OAuth2AuthError> {
        // Implement your own validation logic
        // (JWT decode, database lookup, Redis lookup, etc.)
        
        Ok(TokenData::new("user123".to_string(), vec!["read".to_string()]))
    }
}
```

##### Opaque Token Validator

For testing and simple use cases, `OpaqueTokenValidator` is included:

```rust
use ultraapi::oauth2::OpaqueTokenValidator;

// Add tokens
let validator = OpaqueTokenValidator::new()
    .add_token("valid-token-1", "user1", vec!["read".to_string()])
    .add_token("valid-token-2", "user2", vec!["read".to_string(), "write".to_string()]);

// Validate tokens
let result = validator.validate("valid-token-1").await;
match result {
    Ok(token_data) => {
        println!("User: {}", token_data.sub);
        println!("Scopes: {:?}", token_data.scopes());
    }
    Err(e) => {
        println!("Invalid token: {}", e);
    }
}

// Validate scopes
let token_data = validator.validate("valid-token-2").await.unwrap();
let result = validator.validate_scopes(&token_data, &["read".to_string()]);
// result Ok if user has "read" scope
```

##### Included Types

| Type | Description |
|---|---|
| `OAuth2PasswordRequestForm` | Password flow request form |
| `TokenResponse` | Success token response |
| `OAuth2ErrorResponse` | RFC 6749 compliant error response |
| `TokenData` | Validated token data |
| `OAuth2AuthError` | Token validation error |
| `OAuth2TokenValidator` | Validator trait |
| `OpaqueTokenValidator` | Example opaque token validator implementation |
| `create_bearer_auth_error` | Bearer auth error response helper |

##### Relationship between security Attribute and Middleware

When using `#[security("oauth2Password")]`:

1. oauth2Password is added to OpenAPI securityScheme
2. Middleware checks Authorization header and extracts Bearer token
3. Token is passed to route as `OAuth2PasswordBearer` dependency object
4. When using custom validator, configure `AuthLayer` or `AuthValidator`

When scopes are required:
```rust
#[get("/admin")]
#[security("oauth2Password:admin")]
async fn admin_endpoint(token: OAuth2PasswordBearer) -> String {
    // "admin" scope required
    format!("Admin access for: {}", token.0)
}
```

- `#[response_class("json"|"html"|"text"|"binary"|"stream"|"xml")]`: content-type
- `#[response_model(...)]`: response shaping (include/exclude/by_alias)
- `#[summary("...")]`: OpenAPI summary
- `#[external_docs(url = "...", description = "...")]`: OpenAPI externalDocs
- `#[deprecated]`: OpenAPI deprecated

### Attributes for Model Fields

The following attributes are available for fields in structs with `#[api_model]`:

- `#[read_only]`: Field included only in responses, not in requests (outputs `readOnly: true` in OpenAPI)
- `#[write_only]`: Field included only in requests, not in responses (outputs `writeOnly: true` in OpenAPI)
- `#[alias("name")]`: Specify field serialization name (equivalent to serde's `rename`)

#### read_only / write_only Usage Example

```rust
use ultraapi::prelude::*;

/// User creation request (password only needed in request)
#[api_model]
#[derive(Debug, Clone)]
struct CreateUser {
    /// Username
    name: String,

    /// Password (request only, not returned in response)
    #[write_only]
    password: String,
}

/// User response (ID only returned in response)
#[api_model]
#[derive(Debug, Clone)]
struct User {
    /// User ID (response only)
    #[read_only]
    id: i64,

    /// Username
    name: String,
}
```

- Fields with `#[read_only]` are ignored during request body deserialization
- Fields with `#[write_only]` are excluded during response serialization
- OpenAPI Schema properties output `readOnly: true` / `writeOnly: true` respectively

## Swagger UI / Docs

Default is Embedded (Scalar). To load Swagger UI from CDN:

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new().swagger_cdn("https://unpkg.com/swagger-ui-dist@5");
```

## Webhooks and Callbacks (OpenAPI)

UltraAPI supports OpenAPI 3.1 **webhooks** and **callbacks**.

- `webhooks` outputs to top-level `webhooks` in OpenAPI spec
- `callbacks` outputs to `callbacks` of specific operations

These APIs add **output to OpenAPI** (they do not register to runtime router).
However, whether routes are ultimately exposed depends on the **app's routing method**.

- **explicit routing (using `.include(...)`)**: Routes not included are not registered at runtime
- **implicit routing (using inventory full registration)**: Routes defined with `#[get]`/`#[post]` etc. are registered at runtime

If you want to "include only in OpenAPI", use explicit routing and don't `include(...)` webhook/callback routes.

### Webhooks

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone)]
struct PaymentEvent {
    event_type: String,
    amount: f64,
}

#[post("/webhooks/payment")]
#[tag("webhooks")]
async fn payment_webhook(body: PaymentEvent) -> PaymentEvent {
    body
}

let app = UltraApiApp::new()
    .webhook("payment", __HAYAI_ROUTE_PAYMENT_WEBHOOK);
```

### Callbacks

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone)]
struct Subscription {
    id: i64,
    plan: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct SubscriptionEvent {
    event_type: String,
    subscription_id: i64,
}

#[post("/subscriptions")]
async fn create_subscription(body: Subscription) -> Subscription {
    body
}

#[post("/webhooks/subscription")]
async fn subscription_callback(body: SubscriptionEvent) -> SubscriptionEvent {
    body
}

let app = UltraApiApp::new().callback(
    __HAYAI_ROUTE_CREATE_SUBSCRIPTION,
    "subscriptionEvent",
    "{$request.body#/callbackUrl}",
    __HAYAI_ROUTE_SUBSCRIPTION_CALLBACK,
);
```

## Validation

For types with `#[api_model]`, the following attributes are available:

- `#[validate(email)]`
- `#[validate(min_length = N)]`
- `#[validate(max_length = N)]`
- `#[validate(minimum = N)]`
- `#[validate(maximum = N)]`
- `#[validate(pattern = "...")]`
- `#[validate(min_items = N)]`

Validation runs automatically during Query/Form/Body extraction, and validation failures return 422 (Unprocessable Entity).

## Dependency Injection (DI)

- `Dep<T>` / `State<T>`: Extract dependencies registered with the app
- `Depends<T>`: FastAPI-style dependencies (function-based)
- `yield_depends`: Dependencies with cleanup (scope: Function/Request)

## Sub Applications (mount)

UltraAPI supports FastAPI-like sub applications.

```rust
use ultraapi::prelude::*;

// Create sub app
let sub_app = UltraApiApp::new()
    .title("Sub API")
    .version("1.0.0");

// Mount to main app
let app = UltraApiApp::new()
    .mount("/api", sub_app);
```

Sub applications have the following characteristics:
- Their own `/docs` and `/openapi.json` endpoints (`/api/docs`, `/api/openapi.json`)
- Sub app routes are not included in main app's OpenAPI (separated)
- Share dependencies with main app

## Static Files

You can serve static files (images, CSS, JS, etc.):

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new()
    .static_files("/static", "./static");
```

- First argument: URL path prefix (e.g., `/static`)
- Second argument: Path to directory to serve

## Templates

You can render HTML templates (Jinja2 format):

```rust
use ultraapi::prelude::*;
use ultraapi::templates::{Templates, template_response};

// Set template directory
let app = UltraApiApp::new()
    .templates_dir("./templates");

// Use templates in handler
#[get("/hello")]
async fn hello(templates: Dep<Templates>) -> impl IntoResponse {
    template_response(&templates, "hello.html", serde_json::json!({ "name": "World" }))
}
```

Template features:
- `Templates::new(dir)` - Create Templates from template directory
- `Templates::render(name, context)` - Render template
- `template_response(templates, name, context)` - Generate HTML response
- `TemplateResponse` type implements `IntoResponse`, automatically sets `text/html` content-type

## StreamingResponse

UltraAPI provides `StreamingResponse`, achieving functionality equivalent to FastAPI's `StreamingResponse`. Use it when returning arbitrary streams as HTTP responses.

### Features

- Accepts any `impl Stream<Item = Result<Bytes, E>>` or `impl Stream<Item = Bytes>`
- Content-Type (media_type) can be specified
- Custom headers can be added
- Status code can be specified
- Error handling: Errors in stream are logged and connection is closed

### Basic Usage

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;

/// Stream endpoint
#[get("/stream")]
async fn stream_handler() -> StreamingResponse {
    let stream = iter([
        Ok::<_, std::convert::Infallible>(Bytes::from("chunk1\n")),
        Ok(Bytes::from("chunk2\n")),
        Ok(Bytes::from("chunk3\n")),
    ]);
    StreamingResponse::from_infallible_stream(stream)
}
```

### Custom Content-Type

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;

/// Text stream
#[get("/stream/text")]
async fn text_stream() -> StreamingResponse {
    let stream = iter([
        Ok(Bytes::from("line1\n")),
        Ok(Bytes::from("line2\n")),
        Ok(Bytes::from("line3\n")),
    ]);
    StreamingResponse::from_infallible_stream(stream)
        .content_type("text/plain")
}
```

### Custom Headers

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;

/// Stream with custom headers
#[get("/stream/headers")]
async fn stream_with_headers() -> StreamingResponse {
    let stream = iter([Ok(Bytes::from("data"))]);
    StreamingResponse::from_infallible_stream(stream)
        .header("X-Custom-Header", "custom-value")
        .header("X-Request-Id", "12345")
}
```

### Custom Status Code

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;
use axum::http::StatusCode;

/// Partial content response
#[get("/stream/partial")]
async fn partial_stream() -> StreamingResponse {
    let stream = iter([Ok(Bytes::from("partial content"))]);
    StreamingResponse::from_infallible_stream(stream)
        .status(StatusCode::PARTIAL_CONTENT)
}
```

### Combining All Options

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;
use axum::http::StatusCode;

/// Stream with full options
#[get("/stream/full")]
async fn full_stream() -> StreamingResponse {
    let stream = iter([Ok(Bytes::from("full response"))]);
    StreamingResponse::from_infallible_stream(stream)
        .content_type("application/json")
        .header("X-Request-Id", "12345")
        .status(StatusCode::OK)
}
```

## Response Cookies

UltraAPI can add Set-Cookie headers to responses using `CookieResponse<T>`. Provides functionality similar to FastAPI's `Response.set_cookie()`.

### Basic Usage

```rust
use ultraapi::prelude::*;

/// Login response
#[api_model]
#[derive(Debug, Clone)]
struct LoginResponse {
    status: String,
}

/// Login page
#[post("/login")]
#[response_class("cookie")]
async fn login() -> CookieResponse<LoginResponse> {
    CookieResponse::new(LoginResponse { status: "ok".to_string() })
        .cookie("session", "abc123")
}
```

### Cookie Options

Use the `cookie_options` method to set options like HttpOnly, Secure, SameSite, Path, Max-Age, Expires:

```rust
use ultraapi::prelude::*;
use time::OffsetDateTime;

/// Secure session cookie
#[post("/login/secure")]
#[response_class("cookie")]
async fn login_secure() -> CookieResponse<LoginResponse> {
    // Set expiration to 7 days from now
    let expires = OffsetDateTime::now_utc() + time::Duration::days(7);
    
    CookieResponse::new(LoginResponse { status: "ok".to_string() })
        .cookie_options("session", "abc123", |opts| {
            opts.http_only()      // Not accessible from JavaScript
                .secure()          // Only sent over HTTPS
                .path("/")         // Valid for entire site
                .max_age(86400)    // Valid for 24 hours
                .expires(expires)  // Or absolute datetime
        })
}
```

### Multiple Cookies

You can set multiple cookies:

```rust
use ultraapi::prelude::*;

#[post("/login")]
#[response_class("cookie")]
async fn login() -> CookieResponse<LoginResponse> {
    CookieResponse::new(LoginResponse { status: "ok".to_string() })
        .cookie("session", "abc123")      // Basic cookie
        .cookie("user_id", "42")         // Multiple cookies
        .cookie_options("theme", "dark", |opts| {
            opts.same_site_lax()  // SameSite=Lax
        })
}
```

### Available Options

- `http_only()` - HttpOnly flag (blocks access from JavaScript)
- `secure()` - Secure flag (only sent over HTTPS)
- `same_site_strict()` - SameSite=Strict
- `same_site_lax()` - SameSite=Lax
- `same_site_none()` - SameSite=None (requires Secure)
- `path(path)` - Cookie path
- `max_age(seconds)` - Relative expiration (seconds)
- `expires(datetime)` - Absolute expiration (time::OffsetDateTime)

## File Upload

UltraAPI supports file upload using the `Multipart` extractor.

### Single File Upload

```rust
use ultraapi::prelude::*;
use axum::extract::Multipart;

/// Upload response
#[api_model]
#[derive(Debug, Clone, Serialize)]
struct UploadResponse {
    filename: String,
    content_type: String,
    size: usize,
}

/// Single file upload endpoint
#[post("/upload")]
#[response_class("json")]
async fn upload_file(multipart: Multipart) -> Result<UploadResponse, ApiError> {
    // Get first file field
    let mut multipart = multipart;
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(f)) if f.file_name().is_some() => break f,
            Ok(Some(_)) => continue, // Skip non-file fields
            Ok(None) => return Err(ApiError::bad_request("File not found".to_string())),
            Err(e) => return Err(ApiError::bad_request(format!("Invalid multipart: {}", e))),
        }
    };

    let filename = field
        .file_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let content_type = field
        .content_type()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let data = field
        .bytes()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

    let size = data.len();

    Ok(UploadResponse {
        filename,
        content_type,
        size,
    })
}
```

### Multiple File Upload

You can upload multiple files with the same field name:

```rust
use ultraapi::prelude::*;
use axum::extract::Multipart;

/// File info
#[api_model]
#[derive(Debug, Clone, Serialize)]
struct FileInfo {
    filename: String,
    content_type: String,
    size: usize,
}

/// Multiple file upload response
#[api_model]
#[derive(Debug, Clone, Serialize)]
struct MultipleUploadResponse {
    files: Vec<FileInfo>,
}

/// Multiple file upload endpoint
#[post("/upload/multiple")]
#[response_class("json")]
async fn upload_multiple_files(multipart: Multipart) -> Result<MultipleUploadResponse, ApiError> {
    let mut multipart = multipart;
    let mut files = Vec::new();

    // Process all fields (files)
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::bad_request(format!("Invalid multipart: {}", e)))? {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

        let size = data.len();

        files.push(FileInfo {
            filename,
            content_type,
            size,
        });
    }

    Ok(MultipleUploadResponse { files })
}
```

### File Upload with Metadata

You can send text fields and files simultaneously:

```rust
use ultraapi::prelude::*;
use axum::extract::Multipart;

#[post("/upload/with-meta")]
#[response_class("json")]
async fn upload_file_with_metadata(
    multipart: Multipart,
) -> Result<UploadResponse, ApiError> {
    let mut multipart = multipart;
    
    let mut filename = "default.txt".to_string();
    let mut content_type = "text/plain".to_string();
    let mut size = 0usize;

    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::bad_request(format!("Invalid multipart: {}", e)))? {
        let field_name = field.name().unwrap_or_default();

        if field_name == "description" {
            // Skip description field
            let _ = field.text().await;
        } else if field_name == "file" {
            // Process file field
            filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            content_type = field
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

            size = data.len();
        }
    }

    Ok(UploadResponse {
        filename,
        content_type,
        size,
    })
}
```

## Global Error Handling

In UltraAPI, you can register error handlers to globally handle custom exceptions.

### Defining Custom Exceptions

```rust
use ultraapi::prelude::*;
use axum::http::StatusCode;

/// Custom exception for business logic
#[derive(Debug, Clone)]
struct BusinessException {
    code: String,
    message: String,
}

impl BusinessException {
    fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}
```

### Registering Global Error Handler

```rust
use std::sync::Arc;
use axum::{body::Body, http::Request, response::IntoResponse, http::StatusCode};

fn make_error_handler() -> CustomErrorHandler {
    Arc::new(|_state: AppState, _req: Request<Body>, error: Box<dyn std::any::Any + Send + 'static>| {
        Box::pin(async move {
            // Downcast and handle custom exception types
            if let Some(ex) = error.downcast_ref::<BusinessException>() {
                let body = serde_json::json!({
                    "error": "BusinessError",
                    "code": ex.code,
                    "message": ex.message
                });
                return (StatusCode::BAD_REQUEST, serde_json::to_string(&body).unwrap()).into_response();
            }
            // Default error response
            (StatusCode::INTERNAL_SERVER_ERROR, r#"{"error":"Unknown error"}"#).into_response()
        })
    })
}

// Register error handler when creating app
let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .error_handler_from_arc(make_error_handler());
```

### Enabling Panic Catching

To prevent the entire server from crashing when a panic occurs, you can use the `catch_panic()` method:

```rust
let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .catch_panic();  // Catch panics and return 500 error
```

### Chaining Multiple Options

You can also combine error handler and panic catch:

```rust
let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .error_handler_from_arc(make_error_handler())
    .catch_panic();
```

## Response Compression (GZip / Brotli)

In UltraAPI, you can enable middleware that automatically compresses responses. When the client sends `Accept-Encoding: gzip` or `Accept-Encoding: br`, the server returns a compressed response.

### Basic Usage

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .gzip();  // Enable gzip + brotli compression
```

### FastAPI-compatible GZip Settings (Recommended)

As settings close to FastAPI's `GZipMiddleware`, you can specify `minimum_size` (minimum size to compress) and `content_types` to compress.

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::GZipConfig;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .gzip_config(
        GZipConfig::new()
            .minimum_size(1024)
            .content_types(vec![
                "text/*".to_string(),
                "application/json".to_string(),
            ]),
    );
```

### Custom Settings

You can control compression algorithms individually:

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::CompressionConfig;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .compression(
        CompressionConfig::new()
            .gzip(true)      // Enable gzip
            .brotli(false)   // Disable brotli
            .deflate(false)  // Disable deflate
    );
```

### Behavior

- If client doesn't send `Accept-Encoding` header, compression is not performed
- Small responses (below default threshold) may not be compressed
- Not compressed when `Accept-Encoding: identity`

## TestClient

UltraAPI includes a FastAPI-like `TestClient`. You can test HTTP requests without manually starting a server.

### Basic Usage

```rust
use ultraapi::prelude::*;

// Model definition
#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
}

// Route definition
#[get("/users/{id}")]
async fn get_user(id: i64) -> User {
    User { id, name: "Alice".to_string() }
}

// Test
#[tokio::test]
async fn test_get_user() {
    let app = UltraApiApp::new();
    let client = TestClient::new(app).await;
    
    let response = client.get("/users/42").await;
    assert_eq!(response.status(), 200);
    
    let user: User = response.json().await.unwrap();
    assert_eq!(user.id, 42);
}
```

### Supported HTTP Methods

- `get(path)` - GET request
- `post(path, &body)` - POST request (JSON)
- `put(path, &body)` - PUT request (JSON)
- `delete(path)` - DELETE request
- `patch(path, &body)` - PATCH request (JSON)
- `head(path)` - HEAD request
- `client()` - Get underlying `reqwest::Client` (for custom requests)

### Create from UltraApiApp or Router

```rust
// From UltraApiApp
let app = UltraApiApp::new().title("My API");
let client = TestClient::new(app).await;

// From Router
let router = UltraApiApp::new().into_router();
let client = TestClient::new_router(router).await;
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

## Examples

- `examples/ultraapi-example`
- `examples/grpc-example`

## Implemented Features List

### Core Features
- ✅ FastAPI-style route macros (`#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`, `#[head]`, `#[options]`, `#[trace]`)
- ✅ Automatic OpenAPI 3.1 generation
- ✅ Built-in Swagger UI (`/docs`) and ReDoc (`/redoc`)
- ✅ serde/schemars integration for schema generation
- ✅ Automatic validation with `#[validate]` attributes
- ✅ Dependency injection (`Dep<T>`, `State<T>`, `Depends<T>`)
- ✅ Yield dependencies with cleanup (Function/Request scope)
- ✅ Router composition with prefix/tags/security propagation

### Authentication & Security
- ✅ Bearer authentication (JWT)
- ✅ API Key authentication (header/query/cookie)
- ✅ OAuth2 flows (Implicit, Password, Client Credentials, Authorization Code)
- ✅ OpenID Connect
- ✅ Runtime auth enforcement via middleware
- ✅ Scope-based authorization
- ✅ OAuth2 dependency objects (`OAuth2PasswordBearer`, `OptionalOAuth2PasswordBearer`, etc.)
- ✅ OAuth2 production components (`OAuth2PasswordRequestForm`, `TokenResponse`, etc.)

### Response Handling
- ✅ Response model shaping (include/exclude/by_alias)
- ✅ Response class specification (json/html/text/binary/stream/xml)
- ✅ Field-level attributes (`#[read_only]`, `#[write_only]`, `#[alias]`)
- ✅ Custom status codes via `#[status]`
- ✅ Global error handling with custom exceptions
- ✅ Panic catching
- ✅ Response compression (GZip/Brotli)
- ✅ StreamingResponse for streaming data
- ✅ CookieResponse for setting cookies

### Advanced Features
- ✅ Lifespan hooks (startup/shutdown) with 3 usage patterns
- ✅ WebSocket support (`#[ws]`)
- ✅ SSE support (`#[sse]`)
- ✅ Webhooks (OpenAPI 3.1)
- ✅ Callbacks (OpenAPI 3.1)
- ✅ Sub applications (mount)
- ✅ Static files serving
- ✅ Jinja2-style templates
- ✅ File upload (Multipart)
- ✅ TestClient for testing

### Developer Tools
- ✅ CLI (`ultraapi` command) for running applications
- ✅ Development mode
- ✅ Golden tests for OpenAPI parity with FastAPI

## License

MIT
