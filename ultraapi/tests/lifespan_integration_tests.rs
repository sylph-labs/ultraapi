//! Lifespan integration tests
//!
//! IMPORTANT: These tests must be parallel-safe.
//! We avoid global statics and instead use per-test counters injected via AppState.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use ultraapi::prelude::*;

#[derive(Clone)]
struct Counts {
    startup: Arc<AtomicUsize>,
    shutdown: Arc<AtomicUsize>,
}

impl Counts {
    fn new() -> Self {
        Self {
            startup: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn startup_count(&self) -> usize {
        self.startup.load(Ordering::SeqCst)
    }

    fn shutdown_count(&self) -> usize {
        self.shutdown.load(Ordering::SeqCst)
    }
}

fn app_with_counts(counts: Counts) -> UltraApiApp {
    UltraApiApp::new().dep(counts).lifecycle(|lifecycle| {
        lifecycle
            .on_startup(|state| {
                let counts = state.get::<Counts>().expect("Counts dep missing");
                Box::pin(async move {
                    counts.startup.fetch_add(1, Ordering::SeqCst);
                })
            })
            .on_shutdown(|state| {
                let counts = state.get::<Counts>().expect("Counts dep missing");
                Box::pin(async move {
                    counts.shutdown.fetch_add(1, Ordering::SeqCst);
                })
            })
    })
}

/// Test that startup hook runs when creating a TestClient from UltraApiApp.
#[tokio::test]
async fn test_testclient_runs_startup_hook() {
    let counts = Counts::new();
    let app = app_with_counts(counts.clone());

    let client = TestClient::new(app).await;

    // Startup is executed eagerly by TestClient::new
    assert_eq!(counts.startup_count(), 1);

    // Requests should NOT trigger startup again
    for _ in 0..3 {
        let resp = client.get("/docs").await;
        assert_eq!(resp.status(), 200);
    }

    assert_eq!(counts.startup_count(), 1);
}

/// Test that shutdown hook runs when TestClient::shutdown() is called.
#[tokio::test]
async fn test_testclient_runs_shutdown_hook() {
    let counts = Counts::new();
    let app = app_with_counts(counts.clone());

    let client = TestClient::new(app).await;
    assert_eq!(counts.startup_count(), 1);

    client.shutdown().await;

    assert_eq!(counts.shutdown_count(), 1);
}

/// Test that startup runs only once even with multiple requests.
#[tokio::test]
async fn test_startup_runs_once_with_multiple_requests() {
    let counts = Counts::new();
    let app = app_with_counts(counts.clone());

    let client = TestClient::new(app).await;
    assert_eq!(counts.startup_count(), 1);

    for _ in 0..5 {
        let resp = client.get("/docs").await;
        assert_eq!(resp.status(), 200);
    }

    assert_eq!(counts.startup_count(), 1);
}

/// Test that `into_router()` integrates lifespan (lazy startup on first request).
#[tokio::test]
async fn test_into_router_lazy_startup_on_first_request() {
    let counts = Counts::new();
    let app = app_with_counts(counts.clone());

    let router = app.into_router();

    // Startup should NOT run until a request hits the router.
    assert_eq!(counts.startup_count(), 0);

    let client = TestClient::new_router(router).await;
    let resp = client.get("/docs").await;
    assert_eq!(resp.status(), 200);

    // The middleware ensures startup before executing the request.
    assert_eq!(counts.startup_count(), 1);
}

/// Test both hooks together.
#[tokio::test]
async fn test_lifecycle_both_hooks() {
    let counts = Counts::new();
    let app = app_with_counts(counts.clone());

    let client = TestClient::new(app).await;
    assert_eq!(counts.startup_count(), 1);

    client.shutdown().await;

    assert_eq!(counts.shutdown_count(), 1);
}
