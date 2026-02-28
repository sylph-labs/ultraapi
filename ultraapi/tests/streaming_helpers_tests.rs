// Streaming Helpers Tests
// Tests for streaming helper functions - AsyncRead to StreamingResponse

use axum::http::StatusCode;
use bytes::Bytes;
use std::io::Cursor;
use ultraapi::prelude::*;

// --- Test 1: from_reader with in-memory data ---

/// Endpoint that streams data from a Cursor (in-memory)
#[get("/stream/cursor")]
#[response_class("redirect")]
async fn stream_cursor() -> StreamingResponse {
    let data = b"Hello from cursor!".to_vec();
    let cursor = Cursor::new(data);
    StreamingResponse::from_reader(cursor, 8192).content_type("text/plain")
}

// --- Test 2: bytes_stream helper ---

/// Endpoint that uses bytes_stream helper
#[get("/stream/bytes")]
#[response_class("redirect")]
async fn stream_bytes() -> StreamingResponse {
    let chunks = vec![
        Bytes::from("chunk1\n"),
        Bytes::from("chunk2\n"),
        Bytes::from("chunk3\n"),
    ];
    StreamingResponse::from_stream(ultraapi::streaming::bytes_stream(chunks))
        .content_type("text/plain")
}

// --- Test 3: string_stream helper ---

/// Endpoint that uses string_stream helper
#[get("/stream/strings")]
#[response_class("redirect")]
async fn stream_strings() -> StreamingResponse {
    let strings = vec![
        "line1".to_string(),
        "line2".to_string(),
        "line3".to_string(),
    ];
    StreamingResponse::from_stream(ultraapi::streaming::string_stream(strings))
        .content_type("text/plain")
}

// --- Test 4: iter_stream helper ---

/// Endpoint that uses iter_stream helper
#[get("/stream/iter")]
#[response_class("redirect")]
async fn stream_iter() -> StreamingResponse {
    let numbers = vec![1, 2, 3, 4, 5];
    let stream = ultraapi::streaming::iter_stream(numbers, |n| Bytes::from(format!("num:{},", n)));
    StreamingResponse::from_stream(stream).content_type("text/plain")
}

// --- Test 5: from_reader with custom headers ---

/// Endpoint that streams with custom headers
#[get("/stream/headers")]
#[response_class("redirect")]
async fn stream_with_headers() -> StreamingResponse {
    let data = b"data with custom header".to_vec();
    let cursor = Cursor::new(data);
    StreamingResponse::from_reader(cursor, 8192)
        .content_type("application/octet-stream")
        .header("X-Stream-Header", "custom-value")
        .header("X-Another-Header", "another-value")
}

// --- Test 6: from_reader with custom status ---

/// Endpoint that streams with custom status
#[get("/stream/status")]
#[response_class("redirect")]
async fn stream_with_status() -> StreamingResponse {
    let data = b"partial content".to_vec();
    let cursor = Cursor::new(data);
    StreamingResponse::from_reader(cursor, 8192).status(StatusCode::PARTIAL_CONTENT)
}

// --- Helper to spawn app ---

async fn spawn_app() -> String {
    let app = UltraApiApp::new()
        .title("Streaming Helpers Test API")
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
async fn test_from_reader_basic() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream/cursor")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");
    let body = resp.text().await.unwrap();
    assert_eq!(body, "Hello from cursor!");
}

#[tokio::test]
async fn test_bytes_stream() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream/bytes")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");
    let body = resp.text().await.unwrap();
    assert!(body.contains("chunk1"));
    assert!(body.contains("chunk2"));
    assert!(body.contains("chunk3"));
}

#[tokio::test]
async fn test_string_stream() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream/strings"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");
    let body = resp.text().await.unwrap();
    assert!(body.contains("line1"));
    assert!(body.contains("line2"));
    assert!(body.contains("line3"));
}

#[tokio::test]
async fn test_iter_stream() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream/iter")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("num:1,"));
    assert!(body.contains("num:2,"));
    assert!(body.contains("num:3,"));
}

#[tokio::test]
async fn test_from_reader_custom_headers() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream/headers"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Check custom headers
    let custom_header = resp.headers().get("X-Stream-Header").unwrap();
    assert_eq!(custom_header, "custom-value");

    let another_header = resp.headers().get("X-Another-Header").unwrap();
    assert_eq!(another_header, "another-value");
}

#[tokio::test]
async fn test_from_reader_custom_status() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream/status")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
}

#[tokio::test]
async fn test_from_reader_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream/cursor")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");
}

#[tokio::test]
async fn test_from_reader_binary_content_type() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/stream/headers"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
}
