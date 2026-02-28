// Templates full functionality tests

use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
use ultraapi::prelude::*;
use ultraapi::templates::{html_response, response, TemplateResponse, Templates, TemplatesError};

#[tokio::test]
async fn test_templates_from_string_render() {
    let templates = Templates::from_string("Hello, {{ name }}!").unwrap();
    let html = templates
        .render("template", serde_json::json!({ "name": "World" }))
        .unwrap();
    assert_eq!(html, "Hello, World!");
}

#[tokio::test]
async fn test_templates_not_found_error() {
    let templates = Templates::from_string("dummy").unwrap();
    let result = templates.render("nonexistent", serde_json::json!({}));
    assert!(result.is_err());
}

#[tokio::test]
async fn test_templates_add_filter() {
    let mut templates = Templates::from_string("{{ name|upper }}").unwrap();
    templates.add_filter("my_upper", |s: String| s.to_uppercase());

    let html = templates
        .render("template", serde_json::json!({ "name": "hello" }))
        .unwrap();
    assert_eq!(html, "HELLO");
}

#[tokio::test]
async fn test_templates_add_function() {
    let mut templates = Templates::from_string("{{ greet(name) }}").unwrap();
    templates.add_function("greet", |name: String| format!("Hello, {}!", name));

    let html = templates
        .render("template", serde_json::json!({ "name": "World" }))
        .unwrap();
    assert_eq!(html, "Hello, World!");
}

#[tokio::test]
async fn test_templates_add_global() {
    let mut templates = Templates::from_string("{{ site_name }} - {{ year }}").unwrap();
    templates.add_global("site_name", "My Site");
    templates.add_global("year", 2024);

    let html = templates.render("template", serde_json::json!({})).unwrap();
    assert_eq!(html, "My Site - 2024");
}

#[tokio::test]
async fn test_templates_add_globals() {
    let mut templates = Templates::from_string("{{ a }} + {{ b }}").unwrap();
    templates.add_globals(serde_json::json!({
        "a": 1,
        "b": 2
    }));

    let html = templates.render("template", serde_json::json!({})).unwrap();
    assert_eq!(html, "1 + 2");
}

#[tokio::test]
async fn test_templates_has_template() {
    let templates = Templates::from_string("test").unwrap();
    assert!(templates.has_template("template"));
    assert!(!templates.has_template("nonexistent"));
}

#[tokio::test]
async fn test_template_response_content_type_default() {
    let response = TemplateResponse::from_html("<html>test</html>".to_string()).into_response();
    let status = response.status();
    let headers = response.headers();

    assert_eq!(status.as_u16(), 200);
    assert_eq!(
        headers.get("content-type").unwrap().to_str().unwrap(),
        "text/html; charset=utf-8"
    );
}

#[tokio::test]
async fn test_template_response_status_change() {
    // Test that status can be changed from the default 200
    let response = TemplateResponse::from_html("<html>Not Found</html>".to_string())
        .status(StatusCode::NOT_FOUND)
        .into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Verify body is still correct
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"<html>Not Found</html>");
}

#[tokio::test]
async fn test_template_response_custom_status_201() {
    let response = TemplateResponse::from_html("<html>Created</html>".to_string())
        .status(StatusCode::CREATED)
        .into_response();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_template_response_header_addition() {
    let response = TemplateResponse::from_html("<html>test</html>".to_string())
        .header("X-Custom-Header", "custom-value")
        .header("Cache-Control", "no-cache")
        .into_response();

    let headers = response.headers();

    assert_eq!(
        headers.get("x-custom-header").unwrap().to_str().unwrap(),
        "custom-value"
    );
    assert_eq!(
        headers.get("cache-control").unwrap().to_str().unwrap(),
        "no-cache"
    );
}

#[tokio::test]
async fn test_template_response_content_type_override() {
    // Test default content type
    let response = TemplateResponse::from_html("<html>test</html>".to_string()).into_response();
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/html; charset=utf-8"
    );

    // Test custom content type override
    let response = TemplateResponse::from_html("<xml>test</xml>".to_string())
        .content_type("application/xml")
        .into_response();
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/xml"
    );

    // Test plain text content type
    let response = TemplateResponse::from_html("plain text content".to_string())
        .content_type("text/plain")
        .into_response();
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/plain"
    );
}

#[tokio::test]
async fn test_template_response_combined_builder() {
    // Test all builder methods combined
    let response = TemplateResponse::from_html("<html>Complex Response</html>".to_string())
        .status(StatusCode::ACCEPTED)
        .header("X-Request-Id", "12345")
        .content_type("application/xml")
        .into_response();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap(),
        "12345"
    );
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/xml"
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"<html>Complex Response</html>");
}

#[tokio::test]
async fn test_templates_response_method() {
    // Test the FastAPI-compatible response() method
    let templates = Templates::from_string("Hello, {{ name }}!").unwrap();

    let result = templates.response("template", serde_json::json!({ "name": "World" }));
    assert!(result.is_ok());

    let response = result.unwrap().into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"Hello, World!");
}

#[tokio::test]
async fn test_templates_response_method_with_status() {
    let templates = Templates::from_string("Error: {{ message }}").unwrap();

    let response = templates
        .response("template", serde_json::json!({ "message": "Not Found" }))
        .unwrap()
        .status(StatusCode::NOT_FOUND)
        .into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_response_free_function() {
    // Test the free function version
    let templates = Templates::from_string("Hello, {{ name }}!").unwrap();

    let response = response(
        &templates,
        "template",
        serde_json::json!({ "name": "Free Function" }),
    )
    .unwrap()
    .status(StatusCode::CREATED)
    .into_response();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"Hello, Free Function!");
}

#[tokio::test]
async fn test_template_response_getters() {
    let tmpl_response = TemplateResponse::from_html("test body".to_string());

    assert_eq!(tmpl_response.body(), "test body");
    assert_eq!(tmpl_response.status_code(), StatusCode::OK);
    assert_eq!(tmpl_response.get_content_type(), "text/html; charset=utf-8");
}

#[tokio::test]
async fn test_template_response_getters_after_builder() {
    // Test getters after using builder methods (uses clone)
    let tmpl_response =
        TemplateResponse::from_html("test body".to_string()).status(StatusCode::NOT_FOUND);

    assert_eq!(tmpl_response.status_code(), StatusCode::NOT_FOUND);
    assert_eq!(tmpl_response.body(), "test body");
}

#[tokio::test]
async fn test_html_response_helper() {
    // Test the html_response convenience helper
    let response = html_response("<html><body>Static</body></html>")
        .status(StatusCode::OK)
        .header("X-Powered-By", "UltraAPI")
        .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-powered-by")
            .unwrap()
            .to_str()
            .unwrap(),
        "UltraAPI"
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"<html><body>Static</body></html>");
}

#[tokio::test]
async fn test_templates_error_to_api_error() {
    let err = TemplatesError::NotFound("test.html".to_string());
    let api_err = err.to_api_error();

    assert_eq!(api_err.status.as_u16(), 404);
    assert!(api_err.error.contains("test.html"));

    let err2 = TemplatesError::RenderError("bad render".to_string());
    let api_err2 = err2.to_api_error();
    assert_eq!(api_err2.status.as_u16(), 500);
}
