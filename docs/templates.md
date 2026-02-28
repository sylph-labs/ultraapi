# Templates Guide

UltraAPI provides template rendering functionality using [minijinja](https://github.com/mitsuhiko/minijinja), a Rust implementation of Jinja2. This guide covers everything you need to know about using templates in your UltraAPI applications.

## Table of Contents

- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Templates Configuration](#templates-configuration)
- [Using Templates in Handlers](#using-templates-in-handlers)
- [TemplateResponse](#templateresponse)
- [Dependency Injection](#dependency-injection)
- [Custom Filters and Functions](#custom-filters-and-functions)
- [Development vs Production](#development-vs-production)

---

## Installation

No additional dependencies are required. The `minijinja` crate is already included in UltraAPI:

```toml
[dependencies]
ultraapi = "0.1"
minijinja = { version = "2", features = ["loader"] }
```

---

## Basic Usage

### Creating Templates

```rust
use ultraapi::templates::Templates;
use std::path::PathBuf;

// Load templates from a directory
let templates = Templates::new("./templates").unwrap();

// Or create from a string (useful for testing)
let templates = Templates::from_string("Hello, {{ name }}!").unwrap();
```

### Rendering Templates

```rust
use ultraapi::templates::Templates;

// Render a template with context
let html = templates.render("hello.html", serde_json::json!({
    "name": "World",
    "items": ["apple", "banana", "cherry"]
})).unwrap();
```

---

## Templates Configuration

### Setting Templates Directory

Configure templates at application startup:

```rust
use ultraapi::prelude::*;

#[tokio::main]
async fn main() {
    let app = UltraApi::new()
        .templates_dir("./templates");  // Point to your templates directory
    
    // ... run the app
}
```

### Template Directory Structure

```
templates/
├── base.html
├── index.html
├── users/
│   ├── list.html
│   └── detail.html
└── partials/
    ├── header.html
    └── footer.html
```

---

## Using Templates in Handlers

### Method 1: Using `Dep<Templates>`

```rust
use ultraapi::prelude::*;
use ultraapi::templates::{Templates, TemplateResponse};

#[get("/hello/{name}")]
async fn hello(
    name: Path<String>,
    templates: Dep<Templates>,
) -> impl IntoResponse {
    let html = templates.render("hello.html", serde_json::json!({
        "name": name.0
    })).unwrap();
    
    TemplateResponse::from_html(html)
}
```

### Method 2: Using the `template_response` Helper

```rust
use ultraapi::prelude::*;
use ultraapi::templates::{template_response, Templates};

#[get("/greet/{name}")]
async fn greet(
    name: Path<String>,
    templates: Dep<Templates>,
) -> impl IntoResponse {
    template_response(&templates, "hello.html", serde_json::json!({
        "name": name.0
    })).unwrap()
}
```

### Method 3: Using the `response` Method (FastAPI-compatible)

```rust
use ultraapi::prelude::*;
use ultraapi::templates::Templates;
use axum::http::StatusCode;

#[get("/user/{id}")]
async fn get_user(
    id: Path<i32>,
    templates: Dep<Templates>,
) -> impl IntoResponse {
    templates.response("user.html", serde_json::json!({
        "user_id": id.0,
        "name": "John Doe"
    })).unwrap()
        .status(StatusCode::OK)
}
```

---

## TemplateResponse

`TemplateResponse` is a response type that wraps rendered HTML and implements `IntoResponse`. It provides a fluent builder API for customizing the HTTP response.

### Default Behavior

By default, `TemplateResponse` sets:
- **Status**: `200 OK`
- **Content-Type**: `text/html; charset=utf-8`

```rust
use ultraapi::templates::TemplateResponse;

let response = TemplateResponse::from_html("<html>Hello</html>".to_string());
// Status: 200 OK
// Content-Type: text/html; charset=utf-8
```

### Customizing Status Code

```rust
use ultraapi::templates::TemplateResponse;
use axum::http::StatusCode;

let response = TemplateResponse::from_html("<html>Not Found</html>".to_string())
    .status(StatusCode::NOT_FOUND);
```

### Adding Headers

```rust
use ultraapi::templates::TemplateResponse;

let response = TemplateResponse::from_html("<html>Hello</html>".to_string())
    .header("X-Custom-Header", "custom-value")
    .header("Cache-Control", "no-cache");
```

### Customizing Content-Type

```rust
use ultraapi::templates::TemplateResponse;

// Override default content type
let response = TemplateResponse::from_html("<xml>data</xml>".to_string())
    .content_type("application/xml");

// Or plain text
let response = TemplateResponse::from_html("Plain text".to_string())
    .content_type("text/plain");
```

### Full Example

```rust
use ultraapi::templates::TemplateResponse;
use axum::http::StatusCode;

let response = TemplateResponse::from_html("<html>Complex</html>".to_string())
    .status(StatusCode.ACCEPTED)
    .header("X-Request-Id", "12345")
    .content_type("application/xml");
```

### Helper Functions

UltraAPI provides several helper functions:

```rust
use ultraapi::templates::{html_response, response, render_template};

// Create response from pre-rendered HTML
let response = html_response("<html>Static</html>".to_string())
    .status(StatusCode::OK)
    .header("Cache-Control", "public");

// FastAPI-compatible response helper
let response = response(&templates, "page.html", context).unwrap();

// Legacy compatibility
let response = render_template(&templates, "page.html", context).unwrap();
```

---

## Dependency Injection

### How It Works

When you configure `.templates_dir()` in your app, UltraAPI automatically registers `Templates` as a dependency. You can inject it using `Dep<Templates>`:

```rust
use ultraapi::prelude::*;
use ultraapi::templates::Templates;

#[get("/page")]
async fn my_handler(templates: Dep<Templates>) -> impl IntoResponse {
    templates.render("page.html", serde_json::json!({})).unwrap()
}
```

### Injecting into Multiple Handlers

```rust
use ultraapi::prelude::*;
use ultraapi::templates::{Templates, TemplateResponse};

#[get("/")]
async fn home(templates: Dep<Templates>) -> impl IntoResponse {
    template_response(&templates, "index.html", serde_json::json!({
        "title": "Welcome"
    })).unwrap()
}

#[get("/about")]
async fn about(templates: Dep<Templates>) -> impl IntoResponse {
    template_response(&templates, "about.html", serde_json::json!({
        "team": ["Alice", "Bob", "Charlie"]
    })).unwrap()
}
```

---

## Custom Filters and Functions

### Adding Custom Filters

```rust
use ultraapi::templates::Templates;

let mut templates = Templates::new("./templates").unwrap();

// Add a simple filter
templates.add_filter("my_upper", |s: String| s.to_uppercase());

// In your template: {{ name | my_upper }}
```

### Adding Custom Functions

```rust
use ultraapi::templates::Templates;

let mut templates = Templates::new("./templates").unwrap();

// Add a function that takes arguments
templates.add_function("greet", |name: String| format!("Hello, {}!", name));

// In your template: {{ greet("World") }}
```

### Adding Global Variables

```rust
use ultraapi::templates::Templates;

let mut templates = Templates::new("./templates").unwrap();

// Add a single global
templates.add_global("site_name", "My Website");
templates.add_global("year", 2024);

// Or add multiple at once
templates.add_globals(serde_json::json!({
    "version": "1.0.0",
    "debug": true
}));

// In your template: {{ site_name }} - {{ year }}
```

---

## Development vs Production

### Development Mode (Auto-Reload)

Use `new_with_reload()` for development. Templates are reloaded from disk on each render:

```rust
use ultraapi::templates::Templates;

// Development: templates reload on each request
let templates = Templates::new_with_reload("./templates").unwrap();
```

> **Note:** In minijinja 2.x, the path_loader automatically checks for file changes. `new_with_reload()` is provided for API compatibility, but both `new()` and `new_with_reload()` check for changes.

### Production Mode

Use `new()` for production. This avoids filesystem checks:

```rust
use ultraapi::templates::Templates;

// Production: templates loaded once at startup
let templates = Templates::new("./templates").unwrap();
```

### Recommendations

1. **Development**: Use `Templates::new_with_reload("./templates")` - allows editing templates without restarting the server.

2. **Production**: Use `Templates::new("./templates")` - better performance as templates are loaded once.

3. **Testing**: Use `Templates::from_string(...)` - no filesystem required, faster tests.

---

## Example: Complete Handler

Here's a complete example combining all features:

```rust
use ultraapi::prelude::*;
use ultraapi::templates::{response, TemplateResponse};
use axum::http::StatusCode;
use serde_json::json;

#[get("/users/{user_id}")]
async fn get_user(
    user_id: Path<i32>,
    templates: Dep<Templates>,
) -> impl IntoResponse {
    // Using FastAPI-compatible response() method with custom status
    response(&templates, "user.html", json!({
        "user": {
            "id": user_id.0,
            "name": "John Doe",
            "email": "john@example.com"
        }
    })).unwrap()
    .status(StatusCode::OK)
    .header("Cache-Control", "private, max-age=60")
}
```

---

## Error Handling

Templates can return errors. Handle them appropriately:

```rust
use ultraapi::prelude::*;
use ultraapi::templates::{Templates, TemplatesError};

async fn safe_render(templates: &Templates, name: &str, context: serde_json::Value) 
    -> Result<impl IntoResponse, TemplatesError> 
{
    let html = templates.render(name, context)?;
    Ok(TemplateResponse::from_html(html))
}
```

Error types:
- `TemplatesError::NotFound` - Template file not found (404)
- `TemplatesError::LoadError` - Failed to load template (500)
- `TemplatesError::RenderError` - Template rendering failed (500)

Convert to API error:
```rust
let err = TemplatesError::NotFound("page.html".to_string());
let api_err = err.to_api_error(); // Returns ApiError with 404 status
```
