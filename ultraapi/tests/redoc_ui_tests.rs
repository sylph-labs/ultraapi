use ultraapi::axum;
use ultraapi::prelude::*;

// --- App setup ---

async fn spawn_app() -> String {
    let app = UltraApiApp::new()
        .title("Test API")
        .version("0.1.0")
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_app_with_mounted() -> String {
    let sub_app = UltraApiApp::new().title("Sub API").version("0.1.0");

    let app = UltraApiApp::new()
        .title("Main API")
        .version("0.1.0")
        .mount("/api", sub_app)
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_redoc_returns_200() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/redoc")).await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_redoc_returns_html_with_openapi_reference() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/redoc")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("openapi.json"));
    assert!(body.contains("redoc"));
}

#[tokio::test]
async fn test_docs_still_works() {
    let base = spawn_app().await;
    let resp = reqwest::get(format!("{base}/docs")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.to_lowercase().contains("swagger") || body.to_lowercase().contains("scalar"));
}

#[tokio::test]
async fn test_mounted_subapp_redoc_returns_200() {
    let base = spawn_app_with_mounted().await;
    let resp = reqwest::get(format!("{base}/api/redoc")).await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_mounted_subapp_redoc_has_correct_openapi_reference() {
    let base = spawn_app_with_mounted().await;
    let resp = reqwest::get(format!("{base}/api/redoc")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    // Should reference /api/openapi.json
    assert!(body.contains("/api/openapi.json"));
}
