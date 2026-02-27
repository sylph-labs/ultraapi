// Compression Tests
// Tests for gzip/brotli compression middleware

use ultraapi::axum;
use ultraapi::prelude::*;

// Test model
#[api_model]
#[derive(Debug, Clone)]
struct Message {
    content: String,
}

// Helper function to create a large response for compression testing
fn large_content() -> String {
    // Create a response that's large enough to be compressed (> 1024 bytes)
    "This is test content. ".repeat(100)
}

// --- App setup ---

#[get("/compress/gzip")]
async fn compress_gzip() -> Message {
    Message {
        content: large_content(),
    }
}

#[get("/compress/small")]
async fn compress_small() -> Message {
    Message {
        content: "short".to_string(),
    }
}

#[get("/compress/text")]
#[response_class("text")]
async fn compress_text() -> String {
    large_content()
}

#[get("/compress/empty")]
#[response_class("text")]
async fn compress_empty() -> String {
    "".to_string()
}

async fn spawn_app_with_compression() -> String {
    let app = UltraApiApp::new()
        .title("Compression Test API")
        .version("0.1.0")
        .gzip()  // Enable gzip compression
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_app_without_compression() -> String {
    let app = UltraApiApp::new()
        .title("No Compression Test API")
        .version("0.1.0")
        // No compression enabled
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

// --- Tests ---

#[tokio::test]
async fn test_compression_gzip_with_accept_encoding_gzip() {
    let base = spawn_app_with_compression().await;
    
    // Use reqwest with gzip feature enabled, but disable auto-decompression
    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();
    
    let resp = client
        .get(format!("{}/compress/gzip", base))
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    
    // Check that Content-Encoding header is set to gzip
    let content_encoding = resp.headers()
        .get("content-encoding")
        .map(|v| v.to_str().unwrap());
    
    assert!(content_encoding.is_some(), "Content-Encoding should be present");
    assert_eq!(content_encoding.unwrap(), "gzip", "Should be gzip encoded");
    
    // Verify we can decompress the response
    let body = resp.bytes().await.unwrap();
    
    // Manually decompress to verify it's actually gzip compressed
    use flate2::read::GzDecoder;
    use std::io::Read;
    
    let mut decoder = GzDecoder::new(&body[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).unwrap();
    
    // Verify the content is correct
    assert!(decompressed.contains("This is test content"));
}

#[tokio::test]
async fn test_compression_without_accept_encoding() {
    let base = spawn_app_with_compression().await;
    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();
    
    let resp = client
        .get(format!("{}/compress/gzip", base))
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    
    // With Accept-Encoding: identity, response should NOT be compressed
    let content_encoding = resp.headers().get("content-encoding");
    assert!(
        content_encoding.is_none(),
        "Content-Encoding should not be present for identity"
    );
}

#[tokio::test]
async fn test_no_compression_when_disabled() {
    let base = spawn_app_without_compression().await;
    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();
    
    let resp = client
        .get(format!("{}/compress/gzip", base))
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    
    // Without compression enabled, Content-Encoding should NOT be present
    let content_encoding = resp.headers()
        .get("content-encoding");
    
    assert!(content_encoding.is_none(), "Content-Encoding should not be present when compression is disabled");
}

#[tokio::test]
async fn test_compression_small_response() {
    let base = spawn_app_with_compression().await;
    let client = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    
    let resp = client
        .get(format!("{}/compress/small", base))
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    
    // Small responses might not be compressed (depends on threshold)
    // Just verify the response is valid
    let body: Message = resp.json().await.unwrap();
    assert_eq!(body.content, "short");
}

#[tokio::test]
async fn test_compression_text_response() {
    let base = spawn_app_with_compression().await;
    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();
    
    let resp = client
        .get(format!("{}/compress/text", base))
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    
    // Check Content-Encoding
    let content_encoding = resp.headers()
        .get("content-encoding")
        .map(|v| v.to_str().unwrap());
    
    assert!(content_encoding.is_some());
    assert_eq!(content_encoding.unwrap(), "gzip");
}
