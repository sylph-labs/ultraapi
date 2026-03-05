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

#[api_model]
#[derive(Debug, Clone)]
struct UserOptional {
    id: i64,
    name: String,
    nickname: Option<String>,
}

#[api_model]
#[derive(Debug, Clone)]
struct RuntimeResponseModelProfile {
    visits: i64,
    note: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct RuntimeResponseModelPayload {
    id: i64,
    nickname: Option<String>,
    score: i64,
    enabled: bool,
    label: String,
    profile: RuntimeResponseModelProfile,
}

fn default_mode() -> String {
    "standard".to_string()
}

fn default_max_retries() -> i64 {
    3
}

#[api_model]
#[derive(Debug, Clone)]
struct RuntimeDeclaredDefaultsPayload {
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_max_retries")]
    max_retries: i64,
    active: bool,
}

#[api_model]
#[derive(Debug, Clone, Default)]
struct EchoUnsetProfile {
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
}

#[api_model]
#[derive(Debug, Clone)]
struct EchoUnsetPayload {
    id: i64,
    #[serde(default)]
    nickname: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    profile: EchoUnsetProfile,
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

// --- Test 8: File response (basic) ---

/// File endpoint returns file bytes
#[get("/file")]
#[response_class("file")]
async fn file_response() -> FileResponse {
    FileResponse::new(vec![0x00, 0x01, 0x02, 0xFF])
}

// --- Test 9: File response with filename ---

/// File endpoint returns file with filename
#[get("/file/with-name")]
#[response_class("file")]
async fn file_response_with_name() -> FileResponse {
    FileResponse::new(vec![0xDE, 0xAD, 0xBE, 0xEF]).filename("example.bin")
}

// --- Test 10: File response with custom content-type ---

/// File endpoint returns file with custom content-type
#[get("/file/with-content-type")]
#[response_class("file")]
async fn file_response_with_content_type() -> FileResponse {
    FileResponse::new(vec![0x89, 0x50, 0x4E, 0x47]) // PNG magic bytes
        .with_content_type("image/png")
}

// --- Test 11: File response with filename and content-type ---

/// File endpoint returns file with both filename and content-type
#[get("/file/with-all")]
#[response_class("file")]
async fn file_response_with_all() -> FileResponse {
    FileResponse::new(vec![0x89, 0x50, 0x4E, 0x47])
        .filename("image.png")
        .with_content_type("image/png")
}

// --- Test 12: JSON with Result type ---

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
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/html");
    let body = resp.text().await.unwrap();
    assert!(body.contains("<html>"));
    assert!(body.contains("<h1>Hello World</h1>"));
}

#[tokio::test]
async fn test_text_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/text")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");
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

#[tokio::test]
async fn test_file_response_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/file")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
    // No content-disposition header for file without filename
    assert!(resp.headers().get("content-disposition").is_none());
    // Get bytes last (consumes resp)
    let bytes = resp.bytes().await.unwrap();
    assert_eq!(bytes, vec![0x00, 0x01, 0x02, 0xFF]);
}

#[tokio::test]
async fn test_file_response_with_filename() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/file/with-name"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
    // Check content-disposition header first
    let cd = resp.headers().get("content-disposition").unwrap();
    assert!(cd.to_str().unwrap().contains("attachment"));
    assert!(cd.to_str().unwrap().contains("example.bin"));
    // Get bytes last (consumes resp)
    let bytes = resp.bytes().await.unwrap();
    assert_eq!(bytes, vec![0xDE, 0xAD, 0xBE, 0xEF]);
}

#[tokio::test]
async fn test_file_response_with_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/file/with-content-type"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    // Custom content-type should be used
    assert_eq!(resp.headers().get("content-type").unwrap(), "image/png");
    let bytes = resp.bytes().await.unwrap();
    assert_eq!(bytes, vec![0x89, 0x50, 0x4E, 0x47]);
}

#[tokio::test]
async fn test_file_response_with_all() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/file/with-all")).await.unwrap();
    assert_eq!(resp.status(), 200);
    // Custom content-type should be used
    assert_eq!(resp.headers().get("content-type").unwrap(), "image/png");
    // Check content-disposition header with filename (before getting bytes)
    let cd = resp.headers().get("content-disposition").unwrap();
    assert!(cd.to_str().unwrap().contains("attachment"));
    assert!(cd.to_str().unwrap().contains("image.png"));
    // Get bytes last (consumes resp)
    let bytes = resp.bytes().await.unwrap();
    assert_eq!(bytes, vec![0x89, 0x50, 0x4E, 0x47]);
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

#[tokio::test]
async fn test_openapi_file_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();

    // Check /file endpoint
    let get_op = &spec["paths"]["/file"]["get"];
    let response = &get_op["responses"]["200"]["content"];

    assert!(response.get("application/octet-stream").is_some());
    assert!(response.get("application/json").is_none());
}

#[tokio::test]
async fn test_openapi_non_json_media_types_do_not_emit_null_schema() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();

    for (path, content_type) in [
        ("/html", "text/html"),
        ("/text", "text/plain"),
        ("/binary", "application/octet-stream"),
        ("/stream", "application/octet-stream"),
        ("/xml", "application/xml"),
        ("/file", "application/octet-stream"),
    ] {
        let media = &spec["paths"][path]["get"]["responses"]["200"]["content"][content_type];
        assert!(
            media.get("schema") != Some(&serde_json::Value::Null),
            "{path} emitted invalid schema:null for {content_type}"
        );
    }
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
    assert_eq!(
        ResponseClass::Binary.content_type(),
        "application/octet-stream"
    );
}

#[test]
fn test_response_class_stream_content_type() {
    assert_eq!(
        ResponseClass::Stream.content_type(),
        "application/octet-stream"
    );
}

#[test]
fn test_response_class_sse_content_type() {
    assert_eq!(ResponseClass::Sse.content_type(), "text/event-stream");
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

#[test]
fn test_response_class_file_content_type() {
    assert_eq!(
        ResponseClass::File.content_type(),
        "application/octet-stream"
    );
}

// --- FileResponse tests ---

#[test]
fn test_file_response_new() {
    let bytes = vec![0x00, 0x01, 0x02];
    let response = FileResponse::new(bytes.clone());
    assert_eq!(response.bytes(), &bytes);
    assert_eq!(response.get_filename(), None);
    assert_eq!(response.get_content_type(), "application/octet-stream");
}

#[test]
fn test_file_response_builder_with_filename() {
    let response = FileResponse::new(vec![0x00]).filename("test.txt");
    assert_eq!(response.get_filename(), Some(&"test.txt".to_string()));
}

#[test]
fn test_file_response_builder_with_content_type() {
    let response = FileResponse::new(vec![0x00]).with_content_type("image/png");
    assert_eq!(response.get_content_type(), "image/png");
}

#[test]
fn test_file_response_builder_with_all() {
    let response = FileResponse::new(vec![0x00])
        .filename("test.png")
        .with_content_type("image/png");
    assert_eq!(response.get_filename(), Some(&"test.png".to_string()));
    assert_eq!(response.get_content_type(), "image/png");
}

#[test]
fn test_file_response_into_bytes() {
    let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let response = FileResponse::new(bytes.clone());
    assert_eq!(response.into_bytes(), bytes);
}

#[test]
fn test_file_response_from_vec() {
    let bytes = vec![0xCA, 0xFE, 0xBA, 0xBE];
    let response: FileResponse = bytes.clone().into();
    assert_eq!(response.bytes(), &bytes);
}

// --- Redirect Response Tests ---

/// Test redirect endpoint
#[get("/test-redirect")]
#[response_class("redirect")]
async fn test_redirect_handler() -> RedirectResponse {
    RedirectResponse::new("/new-location")
}

/// Redirect endpoint (default 307)
#[get("/redirect")]
#[response_class("redirect")]
async fn redirect_default() -> RedirectResponse {
    RedirectResponse::new("/new-location")
}

/// Redirect endpoint with custom status 301
#[get("/redirect/301")]
#[response_class("redirect")]
async fn redirect_permanent() -> RedirectResponse {
    RedirectResponse::new("https://example.com/new-location").status(301)
}

/// Redirect endpoint with custom status 302
#[get("/redirect/302")]
#[response_class("redirect")]
async fn redirect_found() -> RedirectResponse {
    RedirectResponse::new("/other-page").status(302)
}

// --- JSON endpoint with content_type override via response_model ---

/// JSON endpoint with custom content-type override
#[get("/json/custom-content-type")]
#[response_model(content_type = "text/plain")]
async fn json_custom_content_type() -> User {
    User {
        id: 100,
        name: "Custom Content Type".into(),
    }
}

#[get("/json/exclude-none")]
#[response_model(exclude_none = true)]
async fn json_exclude_none() -> UserOptional {
    UserOptional {
        id: 101,
        name: "Exclude None".into(),
        nickname: None,
    }
}

#[get("/json/exclude-unset")]
#[response_model(exclude_unset = true)]
async fn json_exclude_unset() -> serde_json::Value {
    ultraapi::serde_json::json!({
        "id": 102,
        "name": "Exclude Unset",
        "nickname": null,
        "tags": [],
        "meta": {}
    })
}

#[get("/json/exclude-defaults")]
#[response_model(exclude_defaults = true)]
async fn json_exclude_defaults() -> serde_json::Value {
    ultraapi::serde_json::json!({
        "id": 0,
        "name": "",
        "enabled": false,
        "score": 0,
        "payload": "ok"
    })
}

#[get("/json/exclude-unset/model")]
#[response_model(exclude_unset = true)]
async fn json_exclude_unset_model() -> RuntimeResponseModelPayload {
    RuntimeResponseModelPayload {
        id: 103,
        nickname: None,
        score: 0,
        enabled: false,
        label: String::new(),
        profile: RuntimeResponseModelProfile {
            visits: 0,
            note: String::new(),
        },
    }
}

#[get("/json/exclude-defaults/model")]
#[response_model(exclude_defaults = true)]
async fn json_exclude_defaults_model() -> RuntimeResponseModelPayload {
    RuntimeResponseModelPayload {
        id: 104,
        nickname: None,
        score: 0,
        enabled: false,
        label: String::new(),
        profile: RuntimeResponseModelProfile {
            visits: 0,
            note: String::new(),
        },
    }
}

#[get("/json/exclude-defaults/model/custom")]
#[response_model(exclude_defaults = true)]
async fn json_exclude_defaults_model_custom() -> RuntimeDeclaredDefaultsPayload {
    RuntimeDeclaredDefaultsPayload {
        mode: default_mode(),
        max_retries: default_max_retries(),
        // No declared default metadata for this field, so falsy values are retained.
        active: false,
    }
}

#[post("/json/exclude-unset/echo")]
#[response_model(exclude_unset = true)]
async fn json_exclude_unset_echo(payload: EchoUnsetPayload) -> EchoUnsetPayload {
    payload
}

#[get("/json/nested-include")]
#[response_model(include = {"order_id", "customer": {"email"}, "items": {"__all__": {"sku"}}})]
async fn json_nested_include() -> serde_json::Value {
    ultraapi::serde_json::json!({
        "order_id": 200,
        "customer": {
            "id": 1,
            "email": "nested@example.com",
            "password_hash": "secret"
        },
        "items": [
            {"sku": "A-1", "qty": 2, "internal_code": "x"},
            {"sku": "B-2", "qty": 1, "internal_code": "y"}
        ],
        "internal_note": "private"
    })
}

#[get("/json/nested-include-full-subtree")]
#[response_model(include = {"order_id", "customer", "items": {"__all__": true}})]
async fn json_nested_include_full_subtree() -> serde_json::Value {
    ultraapi::serde_json::json!({
        "order_id": 202,
        "customer": {
            "id": 3,
            "email": "full@example.com",
            "password_hash": "visible"
        },
        "items": [
            {"sku": "Z-1", "qty": 8, "internal_code": "m"},
            {"sku": "Z-2", "qty": 9, "internal_code": "n"}
        ],
        "internal_note": "private"
    })
}

#[get("/json/nested-exclude")]
#[response_model(exclude = {"customer": {"password_hash"}, "items": {"__all__": {"internal_code"}}, "internal_note"})]
async fn json_nested_exclude() -> serde_json::Value {
    ultraapi::serde_json::json!({
        "order_id": 201,
        "customer": {
            "id": 2,
            "email": "exclude@example.com",
            "password_hash": "secret"
        },
        "items": [
            {"sku": "X-1", "qty": 3, "internal_code": "a"},
            {"sku": "Y-2", "qty": 4, "internal_code": "b"}
        ],
        "internal_note": "private"
    })
}

// --- Runtime Tests ---

#[tokio::test]
async fn test_redirect_status_and_location() {
    let base = spawn_app().await;
    // Wait a bit for the server to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    // Disable automatic redirect following to test the redirect response itself
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let resp = client
        .get(format!("{base}/test-redirect"))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let location = resp
        .headers()
        .get("location")
        .map(|h| h.to_str().unwrap().to_string());
    eprintln!("Status: {}, Location: {:?}", status, location);
    // Default is 307 Temporary Redirect
    assert_eq!(status, 307);
    // Check Location header
    assert_eq!(location.unwrap(), "/new-location");
}

#[tokio::test]
async fn test_redirect_301() {
    let base = spawn_app().await;
    // Disable automatic redirect following
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let resp = client
        .get(format!("{base}/redirect/301"))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let location = resp
        .headers()
        .get("location")
        .map(|h| h.to_str().unwrap().to_string());
    // 301 Moved Permanently
    assert_eq!(status, 301);
    // Check Location header
    assert_eq!(location.unwrap(), "https://example.com/new-location");
}

#[tokio::test]
async fn test_redirect_302() {
    let base = spawn_app().await;
    // Disable automatic redirect following
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let resp = client
        .get(format!("{base}/redirect/302"))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let location = resp
        .headers()
        .get("location")
        .map(|h| h.to_str().unwrap().to_string());
    // 302 Found
    assert_eq!(status, 302);
    // Check Location header
    assert_eq!(location.unwrap(), "/other-page");
}

// --- OpenAPI Tests for Redirect ---

#[tokio::test]
async fn test_openapi_redirect_response_headers() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();

    // Check /redirect endpoint
    let get_op = &spec["paths"]["/redirect"]["get"];
    let response = &get_op["responses"]["307"];

    // Redirect is represented as header-centric (Location) without response body content.
    assert!(response.get("content").is_none());
    assert_eq!(response["headers"]["Location"]["schema"]["type"], "string");
}

// --- OpenAPI Tests for content_type override ---

#[tokio::test]
async fn test_openapi_content_type_override() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();

    // Check /json/custom-content-type endpoint
    let get_op = &spec["paths"]["/json/custom-content-type"]["get"];
    let response = &get_op["responses"]["200"]["content"];

    // Should have text/plain (overridden) instead of application/json
    assert!(response.get("text/plain").is_some());
    assert!(response.get("application/json").is_none());
}

#[tokio::test]
async fn test_response_model_exclude_none_runtime() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/exclude-none"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 101);
    assert_eq!(body["name"], "Exclude None");
    assert!(body.get("nickname").is_none());
}

#[tokio::test]
async fn test_response_model_exclude_unset_runtime_keeps_explicit_null_and_empty_values() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/exclude-unset"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 102);
    assert_eq!(body["name"], "Exclude Unset");
    assert_eq!(body.get("nickname"), Some(&serde_json::Value::Null));
    assert_eq!(body.get("tags"), Some(&serde_json::json!([])));
    assert_eq!(body.get("meta"), Some(&serde_json::json!({})));
}

#[tokio::test]
async fn test_response_model_exclude_defaults_runtime_without_metadata_keeps_values() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/exclude-defaults"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 0);
    assert_eq!(body["name"], "");
    assert_eq!(body["enabled"], false);
    assert_eq!(body["score"], 0);
    assert_eq!(body["payload"], "ok");
}

#[tokio::test]
async fn test_response_model_exclude_unset_runtime_keeps_explicit_api_model_values() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/exclude-unset/model"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 103);
    assert_eq!(body.get("nickname"), Some(&serde_json::Value::Null));
    assert_eq!(body["score"], 0);
    assert_eq!(body["enabled"], false);
    assert_eq!(body["label"], "");

    let profile = body
        .get("profile")
        .and_then(serde_json::Value::as_object)
        .expect("profile must be present as object");
    assert_eq!(profile.get("visits"), Some(&serde_json::json!(0)));
    assert_eq!(profile.get("note"), Some(&serde_json::json!("")));
}

#[tokio::test]
async fn test_response_model_exclude_unset_runtime_prunes_omitted_api_model_fields() {
    let base = spawn_app().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/json/exclude-unset/echo"))
        .json(&serde_json::json!({
            "id": 500,
            "profile": {}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 500);
    assert!(body.get("nickname").is_none());
    assert!(body.get("tags").is_none());
    assert_eq!(body.get("profile"), Some(&serde_json::json!({})));
}

#[tokio::test]
async fn test_response_model_exclude_unset_runtime_keeps_explicit_empty_values_from_request() {
    let base = spawn_app().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/json/exclude-unset/echo"))
        .json(&serde_json::json!({
            "id": 501,
            "nickname": null,
            "tags": [],
            "profile": {
                "note": null,
                "labels": []
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 501);
    assert_eq!(body.get("nickname"), Some(&serde_json::Value::Null));
    assert_eq!(body.get("tags"), Some(&serde_json::json!([])));
    assert_eq!(
        body.get("profile"),
        Some(&serde_json::json!({"note": null, "labels": []}))
    );
}

#[tokio::test]
async fn test_response_model_exclude_defaults_runtime_filters_api_model_payload() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/exclude-defaults/model"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], 104);
    // Option<T> fields default to None, so null is removed.
    assert!(body.get("nickname").is_none());
    // Non-defaulted fields should be retained even when value looks falsy.
    assert_eq!(body.get("score"), Some(&serde_json::json!(0)));
    assert_eq!(body.get("enabled"), Some(&serde_json::json!(false)));
    assert_eq!(body.get("label"), Some(&serde_json::json!("")));
    assert_eq!(
        body.get("profile"),
        Some(&serde_json::json!({"visits": 0, "note": ""}))
    );
}

#[tokio::test]
async fn test_response_model_exclude_defaults_runtime_filters_declared_non_falsy_defaults() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/exclude-defaults/model/custom"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.get("mode").is_none());
    assert!(body.get("max_retries").is_none());
    // No declared default metadata => keep falsy value.
    assert_eq!(body.get("active"), Some(&serde_json::json!(false)));
}

#[tokio::test]
async fn test_response_model_nested_include_runtime() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/nested-include"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["order_id"], 200);
    assert!(body.get("internal_note").is_none());

    let customer = body
        .get("customer")
        .and_then(serde_json::Value::as_object)
        .expect("customer should be object");
    assert_eq!(customer.len(), 1);
    assert_eq!(
        customer.get("email"),
        Some(&serde_json::json!("nested@example.com"))
    );

    let items = body
        .get("items")
        .and_then(serde_json::Value::as_array)
        .expect("items should be array");
    assert_eq!(items.len(), 2);
    for item in items {
        let item = item.as_object().expect("item should be object");
        assert_eq!(item.len(), 1);
        assert!(item.get("sku").is_some());
        assert!(item.get("qty").is_none());
        assert!(item.get("internal_code").is_none());
    }
}

#[tokio::test]
async fn test_response_model_nested_include_runtime_with_full_subtree_selectors() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/nested-include-full-subtree"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["order_id"], 202);
    assert!(body.get("internal_note").is_none());

    let customer = body
        .get("customer")
        .and_then(serde_json::Value::as_object)
        .expect("customer should be object");
    assert_eq!(customer.get("id"), Some(&serde_json::json!(3)));
    assert_eq!(
        customer.get("email"),
        Some(&serde_json::json!("full@example.com"))
    );
    assert_eq!(
        customer.get("password_hash"),
        Some(&serde_json::json!("visible"))
    );

    let items = body
        .get("items")
        .and_then(serde_json::Value::as_array)
        .expect("items should be array");
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0],
        serde_json::json!({"sku": "Z-1", "qty": 8, "internal_code": "m"})
    );
    assert_eq!(
        items[1],
        serde_json::json!({"sku": "Z-2", "qty": 9, "internal_code": "n"})
    );
}

#[tokio::test]
async fn test_response_model_nested_exclude_runtime() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/json/nested-exclude"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["order_id"], 201);
    assert!(body.get("internal_note").is_none());

    let customer = body
        .get("customer")
        .and_then(serde_json::Value::as_object)
        .expect("customer should be object");
    assert_eq!(customer.get("id"), Some(&serde_json::json!(2)));
    assert_eq!(
        customer.get("email"),
        Some(&serde_json::json!("exclude@example.com"))
    );
    assert!(customer.get("password_hash").is_none());

    let items = body
        .get("items")
        .and_then(serde_json::Value::as_array)
        .expect("items should be array");
    assert_eq!(items.len(), 2);
    for item in items {
        let item = item.as_object().expect("item should be object");
        assert!(item.get("sku").is_some());
        assert!(item.get("qty").is_some());
        assert!(item.get("internal_code").is_none());
    }
}

// --- ResponseClass::Redirect tests ---

#[test]
fn test_response_class_redirect_content_type() {
    assert_eq!(ResponseClass::Redirect.content_type(), "application/json");
}

// --- RedirectResponse unit tests ---

#[test]
fn test_redirect_response_new() {
    let response = RedirectResponse::new("/test-location");
    assert_eq!(response.get_location(), "/test-location");
    assert_eq!(
        response.get_status(),
        axum::http::StatusCode::TEMPORARY_REDIRECT
    );
}

#[test]
fn test_redirect_response_builder_location() {
    let response = RedirectResponse::new("/initial").location("/new-location");
    assert_eq!(response.get_location(), "/new-location");
}

#[test]
fn test_redirect_response_builder_status() {
    let response = RedirectResponse::new("/location").status(301);
    assert_eq!(
        response.get_status(),
        axum::http::StatusCode::MOVED_PERMANENTLY
    );
}

#[test]
fn test_redirect_response_builder_all() {
    let response = RedirectResponse::new("/start")
        .location("https://example.com/end")
        .status(308);
    assert_eq!(response.get_location(), "https://example.com/end");
    assert_eq!(
        response.get_status(),
        axum::http::StatusCode::PERMANENT_REDIRECT
    );
}
