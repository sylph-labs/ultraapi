// Response Class Tests
// Tests for response_class attribute to specify different content types
// and verify OpenAPI generation matches runtime behavior

use ultraapi::prelude::*;

// Test models
#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
}

// --- Test 1: Default JSON response (backward compatibility) ---

/// Default JSON endpoint (no response_class specified)
#[get("/json/default")]
async fn json_default() -> User {
    User {
        id: 1,
        name: "Default JSON".into(),
    }
}

// --- Test 2: Explicit JSON response ---

/// Explicit JSON endpoint
#[get("/json/explicit")]
#[response_class("json")]
async fn json_explicit() -> User {
    User {
        id: 2,
        name: "Explicit JSON".into(),
    }
}

// --- Test 3: HTML response ---

/// HTML endpoint returns raw HTML string
#[get("/html")]
#[response_class("html")]
async fn html_response() -> String {
    "<html><body><h1>Hello World</h1></body></html>".to_string()
}

// --- Test 4: Plain text response ---

/// Text endpoint returns plain text
#[get("/text")]
#[response_class("text")]
async fn text_response() -> String {
    "This is a plain text response".to_string()
}

// --- Test 5: Binary response ---

/// Binary endpoint returns binary data
#[get("/binary")]
#[response_class("binary")]
async fn binary_response() -> Vec<u8> {
    vec![0x00, 0x01, 0x02, 0xFF]
}

// --- Test 6: Stream response ---

/// Stream endpoint returns streaming data
#[get("/stream")]
#[response_class("stream")]
async fn stream_response() -> String {
    "Streaming response content".to_string()
}

// --- Test 7: XML response ---

/// XML endpoint returns XML string
#[get("/xml")]
#[response_class("xml")]
async fn xml_response() -> String {
    r#"<user><id>1</id><name>XML User</name></user>"#.to_string()
}

// --- Test 8: JSON with Result type ---

/// JSON endpoint with Result return type
#[get("/json/result")]
async fn json_result() -> Result<User, ApiError> {
    Ok(User {
        id: 3,
        name: "Result JSON".into(),
    })
}

// --- Helper to spawn app ---

async fn spawn_app() -> String {
    let app = UltraApiApp::new()
        .title("Response Class Test API")
        .version("0.1.0")
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

// --- Tests ---

#[tokio::test]
async fn test_json_default_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/default")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 1);
    assert_eq!(body["name"], "Default JSON");
}

#[tokio::test]
async fn test_json_explicit_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/explicit")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 2);
    assert_eq!(body["name"], "Explicit JSON");
}

#[tokio::test]
async fn test_html_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/html")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/html"
    );
    let body = resp.text().await.unwrap();
    assert!(body.contains("<html>"));
    assert!(body.contains("<h1>Hello World</h1>"));
}

#[tokio::test]
async fn test_text_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/text")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/plain"
    );
    let body = resp.text().await.unwrap();
    assert_eq!(body, "This is a plain text response");
}

#[tokio::test]
async fn test_binary_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/binary")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
    let bytes = resp.bytes().await.unwrap();
    assert_eq!(bytes, vec![0x00, 0x01, 0x02, 0xFF]);
}

#[tokio::test]
async fn test_stream_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
    let body = resp.text().await.unwrap();
    assert_eq!(body, "Streaming response content");
}

#[tokio::test]
async fn test_xml_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/xml")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/xml"
    );
    let body = resp.text().await.unwrap();
    assert!(body.contains("<user>"));
    assert!(body.contains("<id>1</id>"));
}

// --- OpenAPI Spec Tests ---

#[tokio::test]
async fn test_openapi_json_default_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();
    
    // Check /json/default endpoint
    let get_op = &spec["paths"]["/json/default"]["get"];
    let response = &get_op["responses"]["200"]["content"];
    
    assert!(response.get("application/json").is_some());
}

#[tokio::test]
async fn test_openapi_html_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();
    
    // Check /html endpoint
    let get_op = &spec["paths"]["/html"]["get"];
    let response = &get_op["responses"]["200"]["content"];
    
    assert!(response.get("text/html").is_some());
    // JSON should not be present for HTML response
    assert!(response.get("application/json").is_none());
}

#[tokio::test]
async fn test_openapi_text_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();
    
    // Check /text endpoint
    let get_op = &spec["paths"]["/text"]["get"];
    let response = &get_op["responses"]["200"]["content"];
    
    assert!(response.get("text/plain").is_some());
    assert!(response.get("application/json").is_none());
}

#[tokio::test]
async fn test_openapi_binary_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();
    
    // Check /binary endpoint
    let get_op = &spec["paths"]["/binary"]["get"];
    let response = &get_op["responses"]["200"]["content"];
    
    assert!(response.get("application/octet-stream").is_some());
    assert!(response.get("application/json").is_none());
}

#[tokio::test]
async fn test_openapi_stream_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();
    
    // Check /stream endpoint
    let get_op = &spec["paths"]["/stream"]["get"];
    let response = &get_op["responses"]["200"]["content"];
    
    assert!(response.get("application/octet-stream").is_some());
    assert!(response.get("application/json").is_none());
}

#[tokio::test]
async fn test_openapi_xml_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();
    
    // Check /xml endpoint
    let get_op = &spec["paths"]["/xml"]["get"];
    let response = &get_op["responses"]["200"]["content"];
    
    assert!(response.get("application/xml").is_some());
    assert!(response.get("application/json").is_none());
}

// --- ResponseClass enum tests ---

#[test]
fn test_response_class_json_content_type() {
    assert_eq!(ResponseClass::Json.content_type(), "application/json");
}

#[test]
fn test_response_class_html_content_type() {
    assert_eq!(ResponseClass::Html.content_type(), "text/html");
}

#[test]
fn test_response_class_text_content_type() {
    assert_eq!(ResponseClass::Text.content_type(), "text/plain");
}

#[test]
fn test_response_class_binary_content_type() {
    assert_eq!(ResponseClass::Binary.content_type(), "application/octet-stream");
}

#[test]
fn test_response_class_stream_content_type() {
    assert_eq!(ResponseClass::Stream.content_type(), "application/octet-stream");
}

#[test]
fn test_response_class_xml_content_type() {
    assert_eq!(ResponseClass::Xml.content_type(), "application/xml");
}

#[test]
fn test_response_class_default_is_json() {
    let rc = ResponseClass::default();
    assert_eq!(rc, ResponseClass::Json);
}
