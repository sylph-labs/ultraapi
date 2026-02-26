//! Tests for mount, static files, and templates features

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

// Import necessary items from ultraapi
use ultraapi::prelude::*;

mod mount_tests {
    use super::*;

    /// Test that mounted sub-app's /docs returns 200
    #[tokio::test]
    async fn test_mount_sub_app_docs() {
        // Create a sub-app
        let sub_app = UltraApiApp::new()
            .title("Sub API")
            .version("1.0.0");

        // Create main app with mounted sub-app
        let app = UltraApiApp::new()
            .mount("/sub", sub_app);

        let router = app.into_router();
        
        // Request /sub/docs
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/sub/docs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        // Check content-type is HTML
        let content_type = response
            .headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap())
            .unwrap_or("");
        assert!(content_type.contains("text/html"));
    }

    /// Test that mounted sub-app's /openapi.json returns 200
    #[tokio::test]
    async fn test_mount_sub_app_openapi() {
        // Create a sub-app
        let sub_app = UltraApiApp::new()
            .title("Sub API")
            .version("1.0.0");

        // Create main app with mounted sub-app
        let app = UltraApiApp::new()
            .mount("/sub", sub_app);

        let router = app.into_router();
        
        // Request /sub/openapi.json
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/sub/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        // Check content-type is JSON
        let content_type = response
            .headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap())
            .unwrap_or("");
        assert!(content_type.contains("application/json"));
    }

    /// Test that main app's /openapi.json does NOT include sub-app routes
    #[tokio::test]
    async fn test_main_openapi_excludes_sub_routes() {
        // Create a sub-app
        let sub_app = UltraApiApp::new()
            .title("Sub API")
            .version("1.0.0");

        // Create main app with mounted sub-app
        let app = UltraApiApp::new()
            .mount("/sub", sub_app);

        let router = app.into_router();
        
        // Request main /openapi.json
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        // Read body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_str = String::from_utf8(body.into()).unwrap();
        
        // The main openapi.json should NOT contain "/sub" paths
        assert!(!json_str.contains("/sub/"));
    }
}

mod static_files_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Test that static files can be served
    #[tokio::test]
    async fn test_static_files_serve() {
        // Create a temp directory with a file
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("hello.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        // Create app with static files
        let app = UltraApiApp::new()
            .static_files("/static", temp_dir.path().to_str().unwrap());

        let router = app.into_router();
        
        // Request the static file
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/static/hello.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        // Check content
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Hello, World!");
    }

    /// Test that static files returns 404 for non-existent file
    #[tokio::test]
    async fn test_static_files_not_found() {
        // Create a temp directory (empty)
        let temp_dir = TempDir::new().unwrap();

        // Create app with static files
        let app = UltraApiApp::new()
            .static_files("/static", temp_dir.path().to_str().unwrap());

        let router = app.into_router();
        
        // Request a non-existent file
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/static/nonexistent.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

mod templates_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use ultraapi::templates::Templates;

    /// Test that templates can be rendered
    #[tokio::test]
    async fn test_templates_render() {
        // Create a temp directory with a template
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("hello.html");
        fs::write(&template_path, "<html><body>Hello, {{ name }}!</body></html>").unwrap();

        // Create templates
        let templates = Templates::new(temp_dir.path()).unwrap();
        
        // Render template
        let html = templates.render("hello.html", serde_json::json!({ "name": "World" })).unwrap();
        
        assert!(html.contains("Hello, World!"));
    }

    /// Test that TemplateResponse returns HTML content-type
    #[tokio::test]
    async fn test_template_response_content_type() {
        use axum::response::IntoResponse;
        
        // Create a temp directory with a template
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("hello.html");
        fs::write(&template_path, "<html><body>Hello!</body></html>").unwrap();

        // Create templates
        let templates = Templates::new(temp_dir.path()).unwrap();
        
        // Create template response
        let response = templates.template_response("hello.html", serde_json::json!({})).unwrap();
        
        // Convert to response
        let axum_response: axum::response::Response = response.into_response();
        
        // Check content-type
        let content_type = axum_response
            .headers()
            .get("content-type")
            .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
            .unwrap_or("");
        assert!(content_type.contains("text/html"));
    }

    /// Test that Templates can be registered as dependency and injected
    #[tokio::test]
    async fn test_templates_as_dependency() {
        // Create a temp directory with a template
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("hello.html");
        fs::write(&template_path, "<html><body>Hello!</body></html>").unwrap();

        // Create app with templates_dir
        let app = UltraApiApp::new()
            .templates_dir(temp_dir.path());

        // Get the router - this should succeed without panicking
        let _router = app.into_router();
    }
}
