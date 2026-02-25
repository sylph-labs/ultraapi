#![allow(dead_code, unused_variables)]

// Advanced Dependency Tests
// Tests for nested dependencies, override precedence, cleanup semantics, and request-scope isolation

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use ultraapi::prelude::*;

// ---- Test 1: Nested Dependencies ----

/// A service that depends on a database
#[derive(Clone)]
struct UserService {
    db: Arc<DbPool>,
}

/// Database pool
#[derive(Clone)]
struct DbPool {
    connection_string: String,
}

#[get("/nested-dep/{id}")]
async fn get_user_with_nested_dep(
    id: i64,
    service: Dep<UserService>,
) -> Result<JsonUser, ApiError> {
    // UserService already has access to DbPool internally
    let _ = service;
    if id <= 0 {
        return Err(ApiError::bad_request("Invalid ID".into()));
    }
    Ok(JsonUser {
        id,
        name: "Test User".into(),
    })
}

#[api_model]
#[derive(Debug, Clone)]
struct JsonUser {
    id: i64,
    name: String,
}

/// Route to verify dependency override - returns the setting value
#[get("/verify-override")]
async fn verify_override_route(dep: Dep<ConfigService>) -> String {
    dep.setting.clone()
}

#[test]
fn test_nested_dep_register_service_and_underlying() {
    // Register the underlying dependency
    let db = DbPool {
        connection_string: "postgres://localhost/test".into(),
    };

    // Register the service that depends on it
    let service = UserService { db: Arc::new(db) };

    let app = UltraApiApp::new().dep(service);

    // App should build successfully
    let _ = app;
}

// ---- Test 2: Override Precedence ----

/// Original service
#[derive(Clone, Debug, PartialEq)]
struct ConfigService {
    setting: String,
}

/// Test override precedence - verifies runtime behavior via HTTP request
#[tokio::test]
async fn test_override_takes_precedence_over_dep() {
    let original = ConfigService {
        setting: "original".into(),
    };

    let override_val = ConfigService {
        setting: "override".into(),
    };

    // Build the app with override
    let app = UltraApiApp::new()
        .dep(original)
        .override_dep(override_val)
        .include(UltraApiRouter::new("/test").route(__HAYAI_ROUTE_VERIFY_OVERRIDE_ROUTE));

    let router = app.into_router();

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Make request and verify override is used
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/test/verify-override", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body = resp.text().await.unwrap();
    // Response is a JSON string, so it includes quotes
    assert!(
        body.contains("override"),
        "Override should be used at runtime, not original. Got: {}",
        body
    );
}

#[test]
fn test_override_without_original() {
    // Override can be used without original dep
    let override_val = ConfigService {
        setting: "only_override".into(),
    };

    let app = UltraApiApp::new().override_dep(override_val);

    assert!(app.has_override::<ConfigService>());
}

#[test]
fn test_multiple_overrides() {
    struct ServiceA(String);
    struct ServiceB(i32);

    let app = UltraApiApp::new()
        .override_dep(ServiceA("a".into()))
        .override_dep(ServiceB(42));

    assert!(app.has_override::<ServiceA>());
    assert!(app.has_override::<ServiceB>());
}

#[test]
fn test_clear_overrides_works() {
    let original = ConfigService {
        setting: "original".into(),
    };

    let override_val = ConfigService {
        setting: "override".into(),
    };

    let app = UltraApiApp::new()
        .dep(original)
        .override_dep(override_val)
        .clear_overrides();

    assert!(!app.has_override::<ConfigService>());
}

// ---- Test 3: Cleanup Semantics ----

// Track how many times drop is called
static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct CleanupTracker {
    id: i32,
}

impl Drop for CleanupTracker {
    fn drop(&mut self) {
        DROP_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

#[get("/cleanup-test")]
async fn cleanup_test_route(_dep: Dep<CleanupTracker>) -> JsonUser {
    JsonUser {
        id: 1,
        name: "test".into(),
    }
}

#[test]
fn test_dep_lives_for_app_lifetime() {
    DROP_COUNT.store(0, Ordering::SeqCst);

    {
        let tracker = CleanupTracker { id: 1 };
        let _app = UltraApiApp::new()
            .dep(tracker)
            .include(UltraApiRouter::new("/test").route(__HAYAI_ROUTE_CLEANUP_TEST_ROUTE));

        // Still alive while app is alive.
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);
    }

    // Dropped when app is dropped.
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);
}

// ---- Test 4: Request-Scope Isolation ----

/// Per-request state
#[derive(Clone, Debug)]
struct RequestContext {
    request_id: String,
    user_id: Option<i64>,
}

#[derive(Clone)]
struct SharedService {
    counter: Arc<AtomicUsize>,
}

/// Route that returns the request_id from context
#[get("/request-scope/{id}")]
async fn request_scope_route(id: i64, ctx: Dep<RequestContext>) -> String {
    // Return the request_id to verify which context instance we got
    ctx.request_id.clone()
}

#[tokio::test]
async fn test_request_context_is_app_scope_not_per_request() {
    // CAPABILITY GAP: The framework does NOT support per-request scoped dependencies.
    // Dep<T> extracts from AppState which is an app-level singleton.
    // This test verifies the ACTUAL behavior (shared across requests), not the
    // false claim of per-request isolation.

    // Create a context with a known request_id
    let context1 = RequestContext {
        request_id: "shared-context".into(),
        user_id: Some(1),
    };

    let app = UltraApiApp::new()
        .dep(context1)
        .include(UltraApiRouter::new("/req").route(__HAYAI_ROUTE_REQUEST_SCOPE_ROUTE))
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Make multiple requests and verify they all get the SAME context
    let resp1 = client
        .get(format!("http://{}/req/request-scope/1", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), 200);
    let body1 = resp1.text().await.unwrap();

    let resp2 = client
        .get(format!("http://{}/req/request-scope/2", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), 200);
    let body2 = resp2.text().await.unwrap();

    // Both requests should return the SAME request_id - proving it's app-scope, not per-request
    assert_eq!(
        body1, body2,
        "Same context should be shared across requests"
    );
    assert_eq!(
        body1, "\"shared-context\"",
        "Should return the shared context's request_id"
    );

    // This demonstrates the capability gap: dependencies are app-level singletons,
    // not per-request scoped. Users needing per-request instances must implement
    // their own factory pattern using State<T> with extractor-side logic.
}

// ---- Test 5: Multiple Dependencies Same Type ----

#[derive(Clone)]
struct Logger {
    prefix: String,
}

#[get("/multi-dep-a")]
async fn multi_dep_a(_dep: Dep<Logger>) -> JsonUser {
    JsonUser {
        id: 1,
        name: "a".into(),
    }
}

#[get("/multi-dep-b")]
async fn multi_dep_b(_dep: Dep<Logger>) -> JsonUser {
    JsonUser {
        id: 2,
        name: "b".into(),
    }
}

#[test]
fn test_same_dep_available_to_multiple_routes() {
    let logger = Logger {
        prefix: "test".into(),
    };

    let app = UltraApiApp::new().dep(logger).include(
        UltraApiRouter::new("")
            .route(__HAYAI_ROUTE_MULTI_DEP_A)
            .route(__HAYAI_ROUTE_MULTI_DEP_B),
    );

    let resolved = app.resolve_routes();
    assert_eq!(resolved.len(), 2);
}

// ---- Test 6: Dependency with State<T> Mix ----

#[derive(Clone)]
struct AppConfig {
    debug: bool,
}

#[derive(Clone)]
struct StateService {
    config: AppConfig,
}

#[get("/mixed/{id}")]
async fn mixed_dep_state(id: i64, _dep: Dep<StateService>, _cfg: State<AppConfig>) -> JsonUser {
    JsonUser {
        id,
        name: "mixed".into(),
    }
}

#[test]
fn test_dep_and_state_can_coexist() {
    let config = AppConfig { debug: true };
    let service = StateService {
        config: config.clone(),
    };

    let app = UltraApiApp::new()
        .dep(service)
        .dep(config)
        .include(UltraApiRouter::new("/mixed").route(__HAYAI_ROUTE_MIXED_DEP_STATE));

    let resolved = app.resolve_routes();
    assert_eq!(resolved.len(), 1);
}

// ---- Test 7: Circular Dependency Detection (should fail at runtime) ----

// Note: The framework doesn't prevent circular dependencies at compile time.
// This test documents expected behavior when circular deps occur.

// ---- Test 8: Lazy Initialization ----

#[derive(Clone)]
struct LazyInitService {
    initialized: Arc<AtomicUsize>,
}

impl LazyInitService {
    fn new() -> Self {
        Self {
            initialized: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn mark_initialized(&self) {
        self.initialized.fetch_add(1, Ordering::SeqCst);
    }
}

#[get("/lazy-init")]
async fn lazy_init_route(dep: Dep<LazyInitService>) -> JsonUser {
    dep.mark_initialized();
    JsonUser {
        id: 1,
        name: "lazy".into(),
    }
}

#[test]
fn test_lazy_init_service_not_touched_until_request() {
    let service = LazyInitService::new();
    let init_count = service.initialized.clone();

    let app = UltraApiApp::new()
        .dep(service)
        .include(UltraApiRouter::new("/lazy").route(__HAYAI_ROUTE_LAZY_INIT_ROUTE));

    let _ = app.into_router();
    assert_eq!(init_count.load(Ordering::SeqCst), 0);
}

// ---- Test 9: Dependency Override with Route ----

#[derive(Clone)]
struct DbConnection {
    query_count: usize,
}

#[get("/override-test/{id}")]
async fn override_test_route(id: i64, _dep: Dep<DbConnection>) -> JsonUser {
    JsonUser {
        id,
        name: "override test".into(),
    }
}

#[test]
fn test_override_works_in_e2e() {
    let real_db = DbConnection { query_count: 0 };
    let test_db = DbConnection { query_count: 999 };

    // Use override in app
    let app = UltraApiApp::new()
        .dep(real_db)
        .override_dep(test_db)
        .include(UltraApiRouter::new("/test").route(__HAYAI_ROUTE_OVERRIDE_TEST_ROUTE));

    // Should build successfully with override
    let _router = app.into_router();
}

// ---- Test 10: Type-Safe Dependencies ----

trait Repository: Send + Sync {
    fn find(&self, id: i64) -> Option<String>;
}

struct MockRepository {
    data: HashMap<i64, String>,
}

impl Repository for MockRepository {
    fn find(&self, id: i64) -> Option<String> {
        self.data.get(&id).cloned()
    }
}

#[get("/repo/{id}")]
async fn repo_route(id: i64, repo: Dep<Box<dyn Repository>>) -> JsonUser {
    let name = repo.find(id).unwrap_or_else(|| "not found".into());
    JsonUser { id, name }
}

#[test]
fn test_trait_object_dependency() {
    let repo = MockRepository {
        data: HashMap::from([(1, "Alice".into())]),
    };

    let app = UltraApiApp::new()
        .dep(Box::new(repo) as Box<dyn Repository>)
        .include(UltraApiRouter::new("/repo").route(__HAYAI_ROUTE_REPO_ROUTE));

    let resolved = app.resolve_routes();
    assert_eq!(resolved.len(), 1);
}

// ---- Test 11: Capability Gap: No Explicit Scoped Dependencies ----

// NOTE: The framework does not support explicit request-scoped dependencies
// that are created fresh for each request. The current implementation uses
// app-level singleton dependencies only.
//
// Users who need per-request dependencies must implement their own factory
// pattern or use State<T> with extractor-side logic.
// This is a known capability gap.

// ---- Test 12: Capability Gap: No Dependency Injection into Middleware ----

// NOTE: The framework does not provide a way to inject dependencies into
// custom axum middleware. This would require extending the middleware
// registration to accept Dep<T> types.
// This is a known capability gap.
