//! Template rendering support using minijinja
//!
//! This module provides template rendering functionality similar to FastAPI's template support.

use axum::{
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
};
use minijinja::Environment;
use serde::Serialize;
use std::path::Path;

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

        // Set some common filters
        // (minijinja has built-in filters like `upper`, `lower`, etc.)

        Ok(Self { env })
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
    /// This is a convenient method that returns a response with `text/html` content-type.
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
}

/// Error type for template operations
#[derive(Debug)]
pub enum TemplatesError {
    /// Failed to load template directory
    LoadError(String),
    /// Failed to render template
    RenderError(String),
}

impl std::fmt::Display for TemplatesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplatesError::LoadError(msg) => write!(f, "Template load error: {}", msg),
            TemplatesError::RenderError(msg) => write!(f, "Template render error: {}", msg),
        }
    }
}

impl std::error::Error for TemplatesError {}

impl From<minijinja::Error> for TemplatesError {
    fn from(e: minijinja::Error) -> Self {
        TemplatesError::LoadError(e.to_string())
    }
}

/// HTML response from template rendering
///
/// This type implements `IntoResponse` and automatically sets the content-type to `text/html`.
pub struct TemplateResponse(String);

impl TemplateResponse {
    /// Create a new TemplateResponse from rendered HTML
    pub fn from_html(html: String) -> Self {
        TemplateResponse(html)
    }
}

impl IntoResponse for TemplateResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, [(CONTENT_TYPE, "text/html")], self.0).into_response()
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
