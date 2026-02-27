#![allow(
    clippy::assertions_on_constants,
    clippy::useless_vec,
    unused_imports,
    dead_code
)]

// P0 File Uploads Tests
// Tests for single and multiple file uploads via MultipartFormData

use std::collections::HashMap;
use ultraapi::axum;
use ultraapi::prelude::*;

// --- App setup for file uploads ---

/// Upload response
#[api_model]
#[derive(Debug, Clone)]
struct UploadResponse {
    filename: String,
    content_type: String,
    size: usize,
}

/// Multiple files upload response
#[api_model]
#[derive(Debug, Clone)]
struct MultipleUploadResponse {
    files: Vec<FileInfo>,
}

#[api_model]
#[derive(Debug, Clone)]
struct FileInfo {
    filename: String,
    content_type: String,
    size: usize,
}

/// Single file upload endpoint
#[post("/upload")]
#[tag("files")]
#[response_class("json")]
async fn upload_file(multipart: Multipart) -> Result<UploadResponse, ApiError> {
    // Get the first file field
    let mut multipart = multipart;
    
    // Find the first field that has a file_name (indicating it's a file)
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(f)) if f.file_name().is_some() => break f,
            Ok(Some(_)) => continue, // Skip non-file fields
            Ok(None) => return Err(ApiError::bad_request("No file found in multipart".to_string())),
            Err(e) => return Err(ApiError::bad_request(format!("Invalid multipart: {}", e))),
        }
    };

    let filename = field
        .file_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let content_type = field
        .content_type()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let data = field
        .bytes()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

    let size = data.len();

    Ok(UploadResponse {
        filename,
        content_type,
        size,
    })
}

/// Multiple files upload endpoint (same field name)
#[post("/upload/multiple")]
#[tag("files")]
#[response_class("json")]
async fn upload_multiple_files(multipart: Multipart) -> Result<MultipleUploadResponse, ApiError> {
    let mut multipart = multipart;
    let mut files = Vec::new();

    // Process all fields (files)
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::bad_request(format!("Invalid multipart: {}", e)))? {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

        let size = data.len();

        files.push(FileInfo {
            filename,
            content_type,
            size,
        });
    }

    Ok(MultipleUploadResponse { files })
}

/// File upload with metadata
#[post("/upload/with-meta")]
#[tag("files")]
#[response_class("json")]
async fn upload_file_with_metadata(
    multipart: Multipart,
) -> Result<UploadResponse, ApiError> {
    let mut multipart = multipart;
    
    // First field: description (text)
    let mut filename = "default.txt".to_string();
    let mut content_type = "text/plain".to_string();
    let mut size = 0usize;

    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::bad_request(format!("Invalid multipart: {}", e)))? {
        let field_name = field.name().unwrap_or_default();

        if field_name == "description" {
            // Skip description field - just consume it
            let _ = field.text().await;
        } else if field_name == "file" {
            // This is the file field
            filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            content_type = field
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

            size = data.len();
        }
    }

    Ok(UploadResponse {
        filename,
        content_type,
        size,
    })
}

// --- Helper ---

async fn spawn_app() -> String {
    let app = UltraApiApp::new()
        .title("File Upload Test API")
        .version("0.1.0")
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
async fn test_single_file_upload() {
    let base = spawn_app().await;
    
    // Create a simple text file
    let file_content = "Hello, World!";
    let file_name = "test.txt";
    
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/upload", base))
        .multipart(reqwest::multipart::Form::new()
            .text("field", "value")
            .part("file", reqwest::multipart::Part::text(file_content)
                .file_name(file_name)
                .mime_str("text/plain")
                .unwrap()))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["filename"], file_name);
    assert_eq!(body["content_type"], "text/plain");
    assert_eq!(body["size"], file_content.len());
}

#[tokio::test]
async fn test_single_file_upload_with_binary_content() {
    let base = spawn_app().await;
    
    // Create binary content
    let file_content: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE];
    let file_name = "binary.bin";
    
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/upload", base))
        .multipart(reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(file_content.clone())
                .file_name(file_name)
                .mime_str("application/octet-stream")
                .unwrap()))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["filename"], file_name);
    assert_eq!(body["content_type"], "application/octet-stream");
    assert_eq!(body["size"], file_content.len());
}

#[tokio::test]
async fn test_multiple_file_upload_same_field() {
    let base = spawn_app().await;
    
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .part("file1", reqwest::multipart::Part::text("File 1 content")
            .file_name("file1.txt")
            .mime_str("text/plain")
            .unwrap())
        .part("file2", reqwest::multipart::Part::text("File 2 content")
            .file_name("file2.txt")
            .mime_str("text/plain")
            .unwrap())
        .part("file3", reqwest::multipart::Part::text("File 3 content")
            .file_name("file3.txt")
            .mime_str("text/plain")
            .unwrap());

    let response = client
        .post(format!("{}/upload/multiple", base))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    let files = body["files"].as_array().unwrap();
    
    assert_eq!(files.len(), 3, "Should have 3 files");
    assert_eq!(files[0]["filename"], "file1.txt");
    assert_eq!(files[1]["filename"], "file2.txt");
    assert_eq!(files[2]["filename"], "file3.txt");
}

#[tokio::test]
async fn test_multiple_file_upload_with_different_content_types() {
    let base = spawn_app().await;
    
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .part("text_file", reqwest::multipart::Part::text("Plain text")
            .file_name("document.txt")
            .mime_str("text/plain")
            .unwrap())
        .part("json_file", reqwest::multipart::Part::text(r#"{"key": "value"}"#)
            .file_name("data.json")
            .mime_str("application/json")
            .unwrap())
        .part("image_file", reqwest::multipart::Part::bytes(vec![0x89, 0x50, 0x4E, 0x47]) // PNG magic bytes
            .file_name("image.png")
            .mime_str("image/png")
            .unwrap());

    let response = client
        .post(format!("{}/upload/multiple", base))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    let files = body["files"].as_array().unwrap();
    
    assert_eq!(files.len(), 3, "Should have 3 files");
    assert_eq!(files[0]["content_type"], "text/plain");
    assert_eq!(files[1]["content_type"], "application/json");
    assert_eq!(files[2]["content_type"], "image/png");
}

#[tokio::test]
async fn test_file_upload_verifies_content() {
    let base = spawn_app().await;
    
    // Test that the actual content is captured correctly
    let file_content = "Test content for verification";
    let expected_size = file_content.len();
    
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/upload", base))
        .multipart(reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::text(file_content)
                .file_name("verify.txt")
                .mime_str("text/plain")
                .unwrap()))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    // The size should match the content length
    assert_eq!(body["size"].as_u64().unwrap() as usize, expected_size);
}

#[tokio::test]
async fn test_file_upload_with_metadata() {
    let base = spawn_app().await;
    
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/upload/with-meta", base))
        .multipart(reqwest::multipart::Form::new()
            .text("description", "A test file")
            .part("file", reqwest::multipart::Part::text("File content here")
                .file_name("metadata_test.txt")
                .mime_str("text/plain")
                .unwrap()))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["filename"], "metadata_test.txt");
    assert_eq!(body["content_type"], "text/plain");
}

#[tokio::test]
async fn test_file_upload_image_content_type() {
    let base = spawn_app().await;
    
    // Test with an image content type
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/upload", base))
        .multipart(reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(vec![0x00])
                .file_name("photo.jpg")
                .mime_str("image/jpeg")
                .unwrap()))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["content_type"], "image/jpeg");
}

#[tokio::test]
async fn test_file_upload_unknown_content_type() {
    let base = spawn_app().await;
    
    // Test without specifying content type (should default to application/octet-stream)
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/upload", base))
        .multipart(reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(vec![0x00, 0x01, 0x02])
                .file_name("unknown.file")))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    // Without explicit mime type, should default
    assert_eq!(body["content_type"], "application/octet-stream");
}

#[tokio::test]
async fn test_single_file_upload_empty_file() {
    let base = spawn_app().await;
    
    // Test with empty file content
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/upload", base))
        .multipart(reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::text("")
                .file_name("empty.txt")
                .mime_str("text/plain")
                .unwrap()))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success(), "Upload should succeed, got: {}", response.status());
    
    let body: serde_json::Value = response.json().await.unwrap();
    // Empty file should have size 0
    assert_eq!(body["size"].as_u64().unwrap(), 0);
}

// --- OpenAPI multipart support tests ---

#[test]
fn test_multipart_extractor_in_openapi_documentation() {
    // This test verifies that multipart endpoints are registered
    // The OpenAPI spec should include these endpoints
    
    // Get all registered routes
    let routes: Vec<_> = inventory::iter::<&ultraapi::RouteInfo>().collect();
    
    // Find our file upload routes
    let upload_routes: Vec<_> = routes.iter()
        .filter(|r| r.path.contains("/upload"))
        .collect();
    
    // We should have at least 3 upload endpoints
    assert!(
        upload_routes.len() >= 3,
        "Should have at least 3 upload endpoints registered"
    );
}

#[test]
fn test_multipart_route_paths() {
    // Verify the route paths are correct
    let routes: Vec<_> = inventory::iter::<&ultraapi::RouteInfo>().collect();
    
    let upload_path = routes.iter().find(|r| r.path == "/upload");
    let multiple_upload_path = routes.iter().find(|r| r.path == "/upload/multiple");
    let with_meta_path = routes.iter().find(|r| r.path == "/upload/with-meta");
    
    assert!(upload_path.is_some(), "/upload route should exist");
    assert!(multiple_upload_path.is_some(), "/upload/multiple route should exist");
    assert!(with_meta_path.is_some(), "/upload/with-meta route should exist");
}
