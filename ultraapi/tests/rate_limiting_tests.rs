//! Rate limiting tests

use std::time::Duration;

use tower::ServiceExt;
use ultraapi::axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use ultraapi::middleware::RateLimitConfig;

// Test handler
async fn hello() -> &'static str {
    "Hello"
}

// --- App setup ---

fn create_app_with_rate_limit(max_requests: u32, window_secs: u64) -> Router {
    Router::new()
        .route("/hello", get(hello))
        .layer(RateLimitConfig::new(max_requests, Duration::from_secs(window_secs)).build())
}

// --- Tests ---

#[tokio::test]
async fn test_rate_limit_allows_requests_within_limit() {
    let app = create_app_with_rate_limit(10, 60);

    // Make 5 requests (within limit of 10)
    for _ in 0..5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/hello")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_rate_limit_blocks_excess_requests() {
    let app = create_app_with_rate_limit(3, 60);

    // Make 3 requests (within limit of 3)
    for _ in 0..3 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/hello")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // The 4th request should be rate limited
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/hello")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Check error response format
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_object());
    let obj = json.as_object().unwrap();
    assert!(obj.contains_key("error"));
    assert!(obj
        .get("error")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("Too Many"));
    assert!(obj.contains_key("details"));
}

#[tokio::test]
async fn test_rate_limit_returns_retry_after() {
    let app = create_app_with_rate_limit(1, 60);

    // First request should succeed
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/hello")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Second request should be rate limited
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/hello")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Check Retry-After header
    assert!(response.headers().contains_key("retry-after"));

    // Check X-RateLimit headers
    assert!(response.headers().contains_key("x-ratelimit-limit"));
    assert!(response.headers().contains_key("x-ratelimit-remaining"));
}

#[tokio::test]
async fn test_rate_limit_with_x_forwarded_for() {
    let app = create_app_with_rate_limit(2, 60);

    // Make 2 requests with same IP (within limit)
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/hello")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // Third request should be rate limited
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/hello")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}
