//! Session cookies (server-side sessions) tests

use std::time::Duration;

use ultraapi::axum;
use ultraapi::prelude::*;

#[get("/session/set")]
#[response_class("text")]
async fn session_set(session: Session) -> String {
    session.insert("user_id", 123_i64).unwrap();
    "ok".to_string()
}

#[get("/session/get")]
#[response_class("text")]
async fn session_get(session: Session) -> String {
    session
        .get::<i64>("user_id")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn make_app(ttl: Duration) -> axum::Router {
    UltraApiApp::new()
        .title("Session Test")
        .version("0.1.0")
        .session_cookies(SessionConfig::new("dev-secret").ttl(ttl))
        .include(
            UltraApiRouter::new("")
                .route(__ULTRAAPI_ROUTE_SESSION_SET)
                .route(__ULTRAAPI_ROUTE_SESSION_GET),
        )
        .into_router()
}

async fn spawn(app: axum::Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

fn cookie_from_set_cookie(resp: &reqwest::Response) -> String {
    let headers = resp.headers();
    let sid = ultraapi::session::extract_session_cookie(headers, "session_id")
        .expect("Set-Cookie session_id missing");
    format!("session_id={}", sid)
}

#[tokio::test]
async fn test_session_sets_cookie_and_persists() {
    let base = spawn(make_app(Duration::from_secs(60))).await;
    let client = reqwest::Client::new();

    // First request: set a value -> should Set-Cookie
    let r1 = client
        .get(format!("{}/session/set", base))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    assert!(r1.headers().get("set-cookie").is_some());
    let cookie = cookie_from_set_cookie(&r1);

    // Second request: send Cookie -> should read persisted value
    let r2 = client
        .get(format!("{}/session/get", base))
        .header("Cookie", cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);
    let body = r2.text().await.unwrap();
    assert_eq!(body, "123");
}

#[tokio::test]
async fn test_session_ttl_expires() {
    let base = spawn(make_app(Duration::from_millis(50))).await;
    let client = reqwest::Client::new();

    let r1 = client
        .get(format!("{}/session/set", base))
        .send()
        .await
        .unwrap();
    let cookie = cookie_from_set_cookie(&r1);

    tokio::time::sleep(Duration::from_millis(80)).await;

    let r2 = client
        .get(format!("{}/session/get", base))
        .header("Cookie", cookie)
        .send()
        .await
        .unwrap();

    let body = r2.text().await.unwrap();
    assert_eq!(body, "none");
}
