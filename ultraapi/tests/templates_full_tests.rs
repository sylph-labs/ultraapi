// Templates full functionality tests

use ultraapi::prelude::*;
use ultraapi::templates::{Templates, TemplateResponse, TemplatesError};

#[tokio::test]
async fn test_templates_from_string_render() {
    let templates = Templates::from_string("Hello, {{ name }}!").unwrap();
    let html = templates.render("template", serde_json::json!({ "name": "World" })).unwrap();
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
    
    let html = templates.render("template", serde_json::json!({ "name": "hello" })).unwrap();
    assert_eq!(html, "HELLO");
}

#[tokio::test]
async fn test_templates_add_function() {
    let mut templates = Templates::from_string("{{ greet(name) }}").unwrap();
    templates.add_function("greet", |name: String| format!("Hello, {}!", name));
    
    let html = templates.render("template", serde_json::json!({ "name": "World" })).unwrap();
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
async fn test_template_response_content_type() {
    use axum::response::IntoResponse;
    
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
async fn test_templates_error_to_api_error() {
    let err = TemplatesError::NotFound("test.html".to_string());
    let api_err = err.to_api_error();
    
    assert_eq!(api_err.status.as_u16(), 404);
    assert!(api_err.error.contains("test.html"));
    
    let err2 = TemplatesError::RenderError("bad render".to_string());
    let api_err2 = err2.to_api_error();
    assert_eq!(api_err2.status.as_u16(), 500);
}

#[get("/template-test")]
async fn template_test_handler(templates: Dep<Templates>) -> Result<TemplateResponse, TemplatesError> {
    templates.template_response("template", serde_json::json!({ "name": "Test" }))
}

#[tokio::test]
async fn test_templates_in_handler() {
    let templates = Templates::from_string("Hello, {{ name }}!").unwrap();
    
    let app = UltraApiApp::new()
        .title("Templates Test")
        .version("0.1.0")
        .dep(templates)
        .include(UltraApiRouter::new("").route(__HAYAI_ROUTE_TEMPLATE_TEST_HANDLER))
        .into_router();
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });
    
    let resp = reqwest::get(format!("http://{}/template-test", addr)).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/html; charset=utf-8"
    );
    let body = resp.text().await.unwrap();
    assert_eq!(body, "Hello, Test!");
}
