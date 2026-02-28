// Response cache middleware tests

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use ultraapi::axum;
use ultraapi::prelude::*;

#[get("/cache/counter")]
#[response_class("text")]
async fn cache_counter(counter: Dep<Arc<AtomicUsize>>) -> String {
    let n = counter.fetch_add(1, Ordering::SeqCst);
    format!("counter={}", n)
}

#[tokio::test]
async fn test_response_cache_hit_and_miss() {
    let counter = Arc::new(AtomicUsize::new(0));

    let app = UltraApiApp::new()
        .dep(counter.clone())
        .response_cache(ResponseCacheConfig::new().ttl(Duration::from_secs(60)))
        .include(UltraApiRouter::new("").route(__ULTRAAPI_ROUTE_CACHE_COUNTER))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // 1st request should be MISS and increments counter
    let r1 = client
        .get(format!("http://{}/cache/counter", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    assert_eq!(
        r1.headers().get("x-cache").unwrap().to_str().unwrap(),
        "MISS"
    );
    let body1 = r1.text().await.unwrap();

    // 2nd request should be HIT and not increment counter
    let r2 = client
        .get(format!("http://{}/cache/counter", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);
    assert_eq!(
        r2.headers().get("x-cache").unwrap().to_str().unwrap(),
        "HIT"
    );
    let body2 = r2.text().await.unwrap();

    assert_eq!(body1, body2);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_response_cache_ttl_expires() {
    let counter = Arc::new(AtomicUsize::new(0));

    let app = UltraApiApp::new()
        .dep(counter.clone())
        .response_cache(ResponseCacheConfig::new().ttl(Duration::from_millis(50)))
        .include(UltraApiRouter::new("").route(__ULTRAAPI_ROUTE_CACHE_COUNTER))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    let r1 = client
        .get(format!("http://{}/cache/counter", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    assert_eq!(
        r1.headers().get("x-cache").unwrap().to_str().unwrap(),
        "MISS"
    );

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(80)).await;

    let r2 = client
        .get(format!("http://{}/cache/counter", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);
    assert_eq!(
        r2.headers().get("x-cache").unwrap().to_str().unwrap(),
        "MISS"
    );

    // Should have executed handler twice due to expiry
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_response_cache_bypass_with_authorization() {
    let counter = Arc::new(AtomicUsize::new(0));

    let app = UltraApiApp::new()
        .dep(counter.clone())
        .response_cache(ResponseCacheConfig::new().ttl(Duration::from_secs(60)))
        .include(UltraApiRouter::new("").route(__ULTRAAPI_ROUTE_CACHE_COUNTER))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    let r1 = client
        .get(format!("http://{}/cache/counter", addr))
        .header("Authorization", "Bearer secret")
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    assert_eq!(
        r1.headers().get("x-cache").unwrap().to_str().unwrap(),
        "BYPASS"
    );

    let r2 = client
        .get(format!("http://{}/cache/counter", addr))
        .header("Authorization", "Bearer secret")
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);
    assert_eq!(
        r2.headers().get("x-cache").unwrap().to_str().unwrap(),
        "BYPASS"
    );

    // Handler executed twice
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_response_cache_respects_no_store() {
    use axum::http::header;

    let counter = Arc::new(AtomicUsize::new(0));
    let counter2 = counter.clone();

    // Use plain axum Router to craft a response with Cache-Control: no-store
    let router = axum::Router::new()
        .route(
            "/cache/no-store",
            axum::routing::get(move || {
                let counter = counter2.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    ([(header::CACHE_CONTROL, "no-store")], "no-store")
                }
            }),
        )
        .layer(
            ResponseCacheConfig::new()
                .ttl(Duration::from_secs(60))
                .build(),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let client = reqwest::Client::new();

    let r1 = client
        .get(format!("http://{}/cache/no-store", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    assert_eq!(
        r1.headers().get("x-cache").unwrap().to_str().unwrap(),
        "BYPASS"
    );

    let r2 = client
        .get(format!("http://{}/cache/no-store", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);
    assert_eq!(
        r2.headers().get("x-cache").unwrap().to_str().unwrap(),
        "BYPASS"
    );

    assert_eq!(counter.load(Ordering::SeqCst), 2);
}
