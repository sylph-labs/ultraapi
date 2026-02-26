use ultraapi::prelude::*;

/// External docs / summary / deprecated should be reflected in OpenAPI
#[get("/external-docs-test")]
#[summary("External docs test")]
#[deprecated]
#[external_docs(
    url = "https://example.com/docs",
    description = "More details here",
)]
async fn external_docs_test() -> String {
    "ok".to_string()
}

async fn spawn_app() -> String {
    let app = UltraApiApp::new()
        .title("External Docs Test API")
        .version("0.1.0")
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_openapi_operation_has_external_docs_summary_and_deprecated() {
    let base = spawn_app().await;

    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);

    let spec: serde_json::Value = resp.json().await.unwrap();

    let op = &spec["paths"]["/external-docs-test"]["get"];

    assert_eq!(op["summary"], "External docs test");
    assert_eq!(op["deprecated"], true);
    assert_eq!(op["externalDocs"]["url"], "https://example.com/docs");
    assert_eq!(
        op["externalDocs"]["description"],
        "More details here"
    );
}
