//! Template rendering support using minijinja
//!
//! This module provides template rendering functionality similar to FastAPI's template support.
//! It extends minijinja with global filters, functions, and auto-reload capability.

use axum::{
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
};
use minijinja::Environment;
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;

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
    pub fn new_with_reload(dir: impl AsRef<Path>) -> Result<Self, TemplatesError> {
        let mut env = Environment::new();
        env.set_loader(minijinja::path_loader(dir));
        env.set_auto_reload(true);

        Ok(Self { env })
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
        // Use a dummy name for string templates
        env.add_template("template", template_content)
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
    pub fn add_filter<F>(&mut self, name: &str, filter_fn: F)
    where
        F: minijinja::Function + Send + Sync + 'static,
    {
        self.env.add_filter(name, filter_fn);
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
    pub fn add_function<F>(&mut self, name: &str, function_fn: F)
    where
        F: minijinja::Function + Send + Sync + 'static,
    {
        self.env.add_function(name, function_fn);
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
            self.env.add_global(name, val);
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
                self.env.add_global(key, value);
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
        Ok(TemplateResponse(html))
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

/// HTML response from template rendering
///
/// This type implements `IntoResponse` and automatically sets the content-type to `text/html; charset=utf-8`.
pub struct TemplateResponse(String);

impl TemplateResponse {
    /// Create a new TemplateResponse from rendered HTML
    pub fn from_html(html: String) -> Self {
        TemplateResponse(html)
    }
}

impl IntoResponse for TemplateResponse {
    fn into_response(self) -> Response {
        // Set Content-Type to text/html with charset=utf-8
        (StatusCode::OK, [(CONTENT_TYPE, "text/html; charset=utf-8")], self.0).into_response()
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
