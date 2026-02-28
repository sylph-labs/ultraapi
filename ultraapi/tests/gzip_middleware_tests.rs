// GZip Middleware Tests
// FastAPI-compatible gzip middleware behavior

use ultraapi::axum;
use ultraapi::prelude::*;

fn large_text() -> String {
    "hello gzip ".repeat(500) // > 1024 bytes
}

#[get("/gzip/large")]
#[response_class("text")]
async fn gzip_large() -> String {
    large_text()
}

#[get("/gzip/small")]
#[response_class("text")]
async fn gzip_small() -> String {
    "small".to_string()
}

async fn gzip_already_encoded_handler() -> impl ultraapi::axum::response::IntoResponse {
    use ultraapi::axum::http::header;

    // Not actually brotli-compressed; this test only verifies that the middleware
    // will not double-compress when Content-Encoding is already present.
    ([(header::CONTENT_ENCODING, "br")], "already-encoded")
}

async fn spawn_app_with_gzip(config: ultraapi::middleware::GZipConfig) -> String {
    let app = UltraApiApp::new()
        .title("GZip Test API")
        .version("0.1.0")
        .gzip_config(config)
        .into_router()
        .route(
            "/gzip/already",
            axum::routing::get(gzip_already_encoded_handler),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_gzip_not_applied_without_accept_encoding_gzip() {
    let base = spawn_app_with_gzip(ultraapi::middleware::GZipConfig::new().minimum_size(128)).await;

    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();

    let resp = client
        .get(format!("{}/gzip/large", base))
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn test_gzip_applied_with_accept_encoding_gzip() {
    let base = spawn_app_with_gzip(ultraapi::middleware::GZipConfig::new().minimum_size(128)).await;

    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();

    let resp = client
        .get(format!("{}/gzip/large", base))
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    assert_eq!(
        resp.headers()
            .get("content-encoding")
            .unwrap()
            .to_str()
            .unwrap(),
        "gzip"
    );

    // Vary: Accept-Encoding is required
    let vary = resp
        .headers()
        .get_all("vary")
        .iter()
        .map(|v| v.to_str().unwrap().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(",");
    assert!(vary.contains("accept-encoding"));

    // Verify we can decompress
    let body = resp.bytes().await.unwrap();

    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(&body[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).unwrap();

    assert!(decompressed.contains("hello gzip"));
}

#[tokio::test]
async fn test_gzip_respects_minimum_size() {
    let base =
        spawn_app_with_gzip(ultraapi::middleware::GZipConfig::new().minimum_size(1024)).await;

    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();

    let resp = client
        .get(format!("{}/gzip/small", base))
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // small body should not be compressed
    assert!(resp.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn test_gzip_respects_content_type_allowlist() {
    // Only allow application/json, but endpoint returns text/plain
    let base = spawn_app_with_gzip(
        ultraapi::middleware::GZipConfig::new()
            .minimum_size(128)
            .content_types(vec!["application/json".to_string()]),
    )
    .await;

    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();

    let resp = client
        .get(format!("{}/gzip/large", base))
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn test_gzip_does_not_double_compress() {
    let base = spawn_app_with_gzip(ultraapi::middleware::GZipConfig::new().minimum_size(1)).await;

    let client = reqwest::Client::builder()
        .gzip(false)
        .brotli(false)
        .build()
        .unwrap();

    let resp = client
        .get(format!("{}/gzip/already", base))
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // Must keep existing encoding
    assert_eq!(
        resp.headers()
            .get("content-encoding")
            .unwrap()
            .to_str()
            .unwrap(),
        "br"
    );
}
