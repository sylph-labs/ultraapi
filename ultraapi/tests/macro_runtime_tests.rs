use std::convert::Infallible;

use ultraapi::prelude::*;

#[ws("/ws-doc-test")]
async fn ws_doc_handler(
    ws: ultraapi::axum::extract::ws::WebSocketUpgrade,
) -> ultraapi::axum::response::Response {
    ws.on_upgrade(|_socket| async move {})
}

#[sse("/sse-doc-test")]
async fn sse_doc_handler(
) -> impl ultraapi::tokio_stream::Stream<Item = Result<ultraapi::axum::response::sse::Event, Infallible>>
{
    ultraapi::tokio_stream::iter(vec![Ok(
        ultraapi::axum::response::sse::Event::default().data("hello")
    )])
}

#[tokio::test]
async fn test_ws_macro_route_is_registered_and_not_404() {
    let app = UltraApiApp::new()
        .include(UltraApiRouter::new("").route(__ULTRAAPI_WS_WS_DOC_HANDLER))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let resp = reqwest::get(format!("http://{}/ws-doc-test", addr))
        .await
        .unwrap();

    // Normal HTTP GET should fail WebSocket upgrade, but route must exist.
    assert_ne!(resp.status(), 404);
}

#[tokio::test]
async fn test_sse_macro_route_streams_events() {
    let app = UltraApiApp::new()
        .include(UltraApiRouter::new("").route(__ULTRAAPI_SSE_SSE_DOC_HANDLER))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });

    let resp = reqwest::get(format!("http://{}/sse-doc-test", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        content_type.contains("text/event-stream"),
        "expected SSE content-type, got: {content_type}"
    );

    let body = resp.text().await.unwrap();
    assert!(body.contains("hello"), "unexpected SSE body: {body}");
}
