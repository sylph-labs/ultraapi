//! Template rendering support using minijinja
//!
//! This module provides template rendering functionality similar to FastAPI's template support.
//! It extends minijinja with global filters, functions, and auto-reload capability.

use axum::{
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use minijinja::functions::Function;
use minijinja::value::{FunctionArgs, FunctionResult, Value};
use minijinja::Environment;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Convert serde_json::Value to minijinja::Value
fn to_minijinja_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::from(None::<()>),
        serde_json::Value::Bool(b) => Value::from(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::from(i)
            } else if let Some(f) = n.as_f64() {
                Value::from(f)
            } else {
                Value::from(n.to_string())
            }
        }
        serde_json::Value::String(s) => Value::from(s),
        serde_json::Value::Array(arr) => {
            Value::from(arr.into_iter().map(to_minijinja_value).collect::<Vec<_>>())
        }
        serde_json::Value::Object(obj) => {
            let map: BTreeMap<String, Value> = obj
                .into_iter()
                .map(|(k, v)| (k, to_minijinja_value(v)))
                .collect();
            Value::from(map)
        }
    }
}

/// Templates struct for rendering Jinja2 templates
///
/// # Example
///
/// ```ignore
/// use ultraapi::templates::Templates;
/// use std::path::PathBuf;
///
/// let templates = Templates::new("./templates").unwrap();
/// let html = templates.render("hello.html", serde_json::json!({ "name": "World" })).unwrap();
/// ```
pub struct Templates {
    env: Environment<'static>,
}

impl Templates {
    /// Create a new Templates instance from a directory path
    ///
    /// The directory should contain Jinja2 template files.
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, TemplatesError> {
        let mut env = Environment::new();
        env.set_loader(minijinja::path_loader(dir));

        // Set some common filters (minijinja has built-in filters like `upper`, `lower`, etc.)
        // Add custom filters below if needed

        Ok(Self { env })
    }

    /// Create a new Templates instance with auto-reload enabled (for development)
    ///
    /// When auto-reload is enabled, templates are reloaded from disk on each render.
    /// This is useful during development but should be disabled in production.
    ///
    /// Note: In minijinja 2.x, the path_loader always checks for file changes when getting templates.
    /// This means templates are effectively reloaded on each render automatically.
    /// However, for production, you should use `Templates::new()` to avoid file system checks.
    pub fn new_with_reload(dir: impl AsRef<Path>) -> Result<Self, TemplatesError> {
        // minijinja 2.x path_loader checks files on each get_template call
        // This provides automatic reload behavior in development
        // For explicit control, you could use a custom loader that caches
        Self::new(dir)
    }

    /// Create a Templates instance from a string (no file system required)
    ///
    /// This is useful for testing or embedding templates in code.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::Templates;
    ///
    /// let templates = Templates::from_string("Hello, {{ name }}!").unwrap();
    /// let html = templates.render("template", serde_json::json!({ "name": "World" })).unwrap();
    /// ```
    pub fn from_string(template_content: &str) -> Result<Self, TemplatesError> {
        let mut env = Environment::new();
        // Use a dummy name for string templates - use owned strings for minijinja 2.x
        env.add_template_owned("template", template_content.to_string())
            .map_err(|e| TemplatesError::LoadError(e.to_string()))?;
        Ok(Self { env })
    }

    /// Add a global filter to the template environment
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::Templates;
    ///
    /// let mut templates = Templates::new("./templates").unwrap();
    /// templates.add_filter("my_filter", |s: String| s.to_uppercase());
    /// ```
    pub fn add_filter<F, Rv, Args>(&mut self, name: &str, filter_fn: F)
    where
        F: Function<Rv, Args> + Send + Sync + 'static,
        Rv: FunctionResult,
        Args: for<'a> FunctionArgs<'a>,
    {
        self.env.add_filter(name.to_string(), filter_fn);
    }

    /// Add a global function to the template environment
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::Templates;
    ///
    /// let mut templates = Templates::new("./templates").unwrap();
    /// templates.add_function("hello", |name: String| format!("Hello, {}!", name));
    /// ```
    pub fn add_function<F, Rv, Args>(&mut self, name: &str, function_fn: F)
    where
        F: Function<Rv, Args> + Send + Sync + 'static,
        Rv: FunctionResult,
        Args: for<'a> FunctionArgs<'a>,
    {
        self.env.add_function(name.to_string(), function_fn);
    }

    /// Add a global variable that will be available in all templates
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::Templates;
    ///
    /// let mut templates = Templates::new("./templates").unwrap();
    /// templates.add_global("site_name", "My Website");
    /// templates.add_global("year", 2024);
    /// ```
    pub fn add_global<V: Serialize>(&mut self, name: &str, value: V) {
        if let Ok(val) = serde_json::to_value(value) {
            self.env
                .add_global(name.to_string(), to_minijinja_value(val));
        }
    }

    /// Add multiple global variables from a JSON value
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::Templates;
    ///
    /// let mut templates = Templates::new("./templates").unwrap();
    /// templates.add_globals(serde_json::json!({
    ///     "site_name": "My Website",
    ///     "year": 2024
    /// }));
    /// ```
    pub fn add_globals(&mut self, globals: serde_json::Value) {
        if let serde_json::Value::Object(map) = globals {
            for (key, value) in map {
                self.env.add_global(key, to_minijinja_value(value));
            }
        }
    }

    /// Render a template with the given context
    ///
    /// # Example
    ///
    /// ```ignore
    /// let html = templates.render("hello.html", serde_json::json!({ "name": "World" })).unwrap();
    /// ```
    pub fn render(&self, name: &str, context: impl Serialize) -> Result<String, TemplatesError> {
        let template = self.env.get_template(name)?;
        // minijinja expects a serde_json::Value for context
        let context = serde_json::to_value(context).map_err(|e| {
            TemplatesError::RenderError(format!("Failed to serialize context: {}", e))
        })?;
        template
            .render(context)
            .map_err(|e| TemplatesError::RenderError(format!("Template render error: {}", e)))
    }

    /// Get a TemplateResponse for the given template and context
    ///
    /// This is a convenient method that returns a response with `text/html; charset=utf-8` content-type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::TemplateResponse;
    ///
    /// // In a handler
    /// async fn hello() -> TemplateResponse {
    ///     TemplateResponse::new("hello.html", serde_json::json!({ "name": "World" }))
    /// }
    /// ```
    pub fn template_response(
        &self,
        name: &str,
        context: impl Serialize,
    ) -> Result<TemplateResponse, TemplatesError> {
        let html = self.render(name, context)?;
        Ok(TemplateResponse::from_html(html))
    }

    /// FastAPI-compatible method to render a template and return a TemplateResponse builder
    ///
    /// This provides a convenient way to create a TemplateResponse with custom status,
    /// headers, or content-type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::Templates;
    /// use axum::http::StatusCode;
    ///
    /// // Basic usage
    /// let response = templates.response("hello.html", serde_json::json!({ "name": "World" }));
    ///
    /// // With custom status
    /// let response = templates.response("error.html", serde_json::json!({}))
    ///     .status(StatusCode::NOT_FOUND);
    ///
    /// // With custom headers
    /// let response = templates.response("page.html", serde_json::json!({}))
    ///     .header("X-Custom-Header", "value");
    /// ```
    pub fn response(
        &self,
        name: &str,
        context: impl Serialize,
    ) -> Result<TemplateResponse, TemplatesError> {
        let html = self.render(name, context)?;
        Ok(TemplateResponse::from_html(html))
    }

    /// Check if a template exists
    pub fn has_template(&self, name: &str) -> bool {
        self.env.get_template(name).is_ok()
    }
}

/// Error type for template operations
#[derive(Debug)]
pub enum TemplatesError {
    /// Failed to load template directory
    LoadError(String),
    /// Failed to render template
    RenderError(String),
    /// Template not found
    NotFound(String),
}

impl TemplatesError {
    /// Convert to ApiError for HTTP responses
    ///
    /// This converts template errors to a format suitable for API responses.
    /// - Template not found -> 404 Not Found
    /// - Load/Render errors -> 500 Internal Server Error
    pub fn to_api_error(&self) -> crate::ApiError {
        match self {
            TemplatesError::NotFound(msg) => crate::ApiError::not_found(msg.clone()),
            TemplatesError::LoadError(msg) => {
                crate::ApiError::internal(format!("Template load error: {}", msg))
            }
            TemplatesError::RenderError(msg) => {
                crate::ApiError::internal(format!("Template render error: {}", msg))
            }
        }
    }
}

impl From<TemplatesError> for crate::ApiError {
    fn from(err: TemplatesError) -> Self {
        err.to_api_error()
    }
}

impl std::fmt::Display for TemplatesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplatesError::LoadError(msg) => write!(f, "Template load error: {}", msg),
            TemplatesError::RenderError(msg) => write!(f, "Template render error: {}", msg),
            TemplatesError::NotFound(msg) => write!(f, "Template not found: {}", msg),
        }
    }
}

impl std::error::Error for TemplatesError {}

impl From<minijinja::Error> for TemplatesError {
    fn from(e: minijinja::Error) -> Self {
        let msg = e.to_string();
        if msg.contains("template not found") || msg.contains("could not find template") {
            TemplatesError::NotFound(msg)
        } else if msg.contains("render") {
            TemplatesError::RenderError(msg)
        } else {
            TemplatesError::LoadError(msg)
        }
    }
}

/// HTML response from template rendering with full response capabilities
///
/// This type implements `IntoResponse` and allows customization of:
/// - HTTP status code
/// - Response headers
/// - Content type
///
/// # Example
///
/// ```ignore
/// use ultraapi::templates::TemplateResponse;
/// use axum::http::StatusCode;
///
/// // Basic usage
/// let response = TemplateResponse::from_html("<html>test</html>".to_string());
///
/// // With custom status
/// let response = TemplateResponse::from_html("<html>not found</html>".to_string())
///     .status(StatusCode::NOT_FOUND);
///
/// // With custom headers
/// let response = TemplateResponse::from_html("<html>test</html>".to_string())
///     .header("X-Custom-Header", "value")
///     .header("Cache-Control", "no-cache");
///
/// // With custom content type
/// let response = TemplateResponse::from_html("<html>test</html>".to_string())
///     .content_type("application/xml");
/// ```
#[derive(Clone, Debug)]
pub struct TemplateResponse {
    /// The HTML body content
    body: String,
    /// HTTP status code (defaults to 200 OK)
    status: StatusCode,
    /// Response headers
    headers: HeaderMap,
    /// Content type (defaults to text/html; charset=utf-8)
    content_type: String,
}

impl TemplateResponse {
    /// Create a new TemplateResponse from rendered HTML
    ///
    /// This is the basic constructor that creates a response with:
    /// - Status: 200 OK
    /// - Content-Type: text/html; charset=utf-8
    /// - No additional headers
    pub fn from_html(html: String) -> Self {
        TemplateResponse {
            body: html,
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            content_type: "text/html; charset=utf-8".to_string(),
        }
    }

    /// Set the HTTP status code for the response
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::TemplateResponse;
    /// use axum::http::StatusCode;
    ///
    /// let response = TemplateResponse::from_html("<html>Not Found</html>".to_string())
    ///     .status(StatusCode::NOT_FOUND);
    /// ```
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Set a response header
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::TemplateResponse;
    ///
    /// let response = TemplateResponse::from_html("<html>test</html>".to_string())
    ///     .header("X-Custom-Header", "value")
    ///     .header("Cache-Control", "no-cache");
    /// ```
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name_header = axum::http::HeaderName::from_bytes(name.into().as_bytes())
            .unwrap_or_else(|_| axum::http::HeaderName::from_static("x-custom"));
        let value_header =
            HeaderValue::from_str(&value.into()).unwrap_or_else(|_| HeaderValue::from_static(""));
        self.headers.insert(name_header, value_header);
        self
    }

    /// Set the content type for the response
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultraapi::templates::TemplateResponse;
    ///
    /// // Set custom content type
    /// let response = TemplateResponse::from_html("<xml>test</xml>".to_string())
    ///     .content_type("application/xml");
    ///
    /// // Override default content type
    /// let response = TemplateResponse::from_html("plain text".to_string())
    ///     .content_type("text/plain");
    /// ```
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }

    /// Get a reference to the response body
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Get a mutable reference to the response body
    pub fn body_mut(&mut self) -> &mut String {
        &mut self.body
    }

    /// Get the current status code
    pub fn status_code(&self) -> StatusCode {
        self.status
    }

    /// Get a reference to the headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get the current content type
    pub fn get_content_type(&self) -> &str {
        &self.content_type
    }
}

impl IntoResponse for TemplateResponse {
    fn into_response(self) -> Response {
        // Build headers: start with user-provided headers, then set content-type
        let mut headers = self.headers.clone();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(&self.content_type)
                .unwrap_or_else(|_| HeaderValue::from_static("text/html; charset=utf-8")),
        );

        (self.status, headers, self.body).into_response()
    }
}

/// Helper to create a TemplateResponse from a Templates instance
///
/// # Example
///
/// ```ignore
/// use ultraapi::templates::{Templates, template_response};
///
/// fn handler(dep: Dep<Templates>) -> impl IntoResponse {
///     template_response(&dep, "hello.html", serde_json::json!({ "name": "World" }))
/// }
/// ```
pub fn template_response(
    templates: &Templates,
    name: &str,
    context: impl Serialize,
) -> Result<TemplateResponse, TemplatesError> {
    templates.template_response(name, context)
}

/// FastAPI-compatible helper to create a TemplateResponse with builder pattern
///
/// This function provides a convenient way to create a TemplateResponse that can be
/// customized with status, headers, and content-type.
///
/// # Example
///
/// ```ignore
/// use ultraapi::templates::{Templates, response};
/// use axum::http::StatusCode;
///
/// fn handler(dep: Dep<Templates>) -> impl IntoResponse {
///     response(&dep, "hello.html", serde_json::json!({ "name": "World" }))
///         .status(StatusCode::CREATED)
///         .header("X-Custom-Header", "value")
/// }
/// ```
pub fn response(
    templates: &Templates,
    name: &str,
    context: impl Serialize,
) -> Result<TemplateResponse, TemplatesError> {
    templates.response(name, context)
}

/// Create a TemplateResponse directly (convenience function)
///
/// This is a convenience function that can be used in handlers.
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
///
/// #[get("/hello")]
/// async fn hello(templates: Dep<Templates>) -> impl IntoResponse {
///     templates::render_template(&templates, "hello.html", serde_json::json!({ "name": "World" }))
/// }
/// ```
pub fn render_template(
    templates: &Templates,
    name: &str,
    context: impl Serialize,
) -> Result<TemplateResponse, TemplatesError> {
    templates.template_response(name, context)
}

/// Create a TemplateResponse directly from HTML string (convenience function)
///
/// This is useful when you have pre-rendered HTML or want to return static HTML
/// with custom status/headers.
///
/// # Example
///
/// ```ignore
/// use ultraapi::templates::html_response;
/// use axum::http::StatusCode;
///
/// #[get("/static")]
/// async fn static_page() -> impl IntoResponse {
///     html_response("<html><body>Static Page</body></html>")
///         .status(StatusCode::OK)
///         .header("Cache-Control", "public, max-age=3600")
/// }
/// ```
pub fn html_response(html: impl Into<String>) -> TemplateResponse {
    TemplateResponse::from_html(html.into())
}
