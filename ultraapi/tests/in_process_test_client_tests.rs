//! In-process TestClient tests
//!
//! These tests verify that the in-process client can execute requests
//! without binding a TCP port.

use ultraapi::axum;
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone)]
struct Ping {
    message: String,
}

#[get("/hello")]
async fn hello() -> String {
    "Hello".to_string()
}

#[post("/echo")]
#[status(200)]
async fn echo(body: Ping) -> Ping {
    body
}

fn app_for_in_process() -> UltraApiApp {
    UltraApiApp::new()
        .title("InProcess")
        .version("0.1.0")
        .include(UltraApiRouter::new("").route(__ULTRAAPI_ROUTE_HELLO))
        .include(UltraApiRouter::new("").route(__ULTRAAPI_ROUTE_ECHO))
}

#[tokio::test]
async fn test_in_process_get_works() {
    let client = TestClient::new_in_process(app_for_in_process()).await;

    let resp = client.get("/hello").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().unwrap(), "\"Hello\"");
}

#[tokio::test]
async fn test_in_process_post_json_works() {
    let client = TestClient::new_in_process(app_for_in_process()).await;

    let resp = client
        .post(
            "/echo",
            &Ping {
                message: "ok".into(),
            },
        )
        .await;
    assert_eq!(resp.status(), 200);

    let body: Ping = resp.json().await.unwrap();
    assert_eq!(body.message, "ok");
}

#[tokio::test]
async fn test_in_process_custom_header_is_applied() {
    use axum::http::HeaderMap;

    async fn header_echo(headers: HeaderMap) -> String {
        headers
            .get("x-test")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("missing")
            .to_string()
    }

    let router = axum::Router::new().route("/header", axum::routing::get(header_echo));
    let client = TestClient::new_router_in_process(router);

    let resp = client
        .request_with_header(
            axum::http::Method::GET,
            "/header",
            None,
            Some(("x-test", "yes")),
        )
        .await;

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().unwrap(), "yes");
}
