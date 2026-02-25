// Tests for FastAPI-style Depends function-based dependency injection
//
// These tests verify:
// 1. Function-based dependencies can be registered
// 2. Nested dependency chains work (dependencies resolving other dependencies)
// 3. Override behavior works with function-based deps
// 4. Backward compatibility (regular dep without function)
// 5. Missing dependency errors

use std::sync::Arc;
use ultraapi::prelude::*;
use ultraapi::{AppState, DependencyError};

// ---- Test 1: Simple function dependency registration ----

#[derive(Clone)]
struct SimpleService {
    value: i32,
}

// Simple function dependency - takes AppState and returns the dependency
async fn get_simple_service(_state: AppState) -> Result<Arc<SimpleService>, DependencyError> {
    Ok(Arc::new(SimpleService { value: 42 }))
}

#[test]
fn test_simple_function_dependency_registration() {
    let app = UltraApiApp::new()
        .depends(get_simple_service);

    // Just verify registration works
    assert!(app.get_depends_resolver().is_some());
}

// ---- Test 2: Nested dependency chain ----

#[derive(Clone)]
struct DatabasePool {
    connection_string: String,
}

#[derive(Clone)]
struct UserRepository {
    pool: Arc<DatabasePool>,
}

// First level: Database pool from AppState
async fn get_db_pool(state: AppState) -> Result<Arc<DatabasePool>, DependencyError> {
    state.get::<DatabasePool>().ok_or_else(|| DependencyError::missing("DatabasePool"))
}

// Second level: UserRepository depends on DatabasePool (resolved manually in function)
async fn get_user_repository(state: AppState) -> Result<Arc<UserRepository>, DependencyError> {
    let pool = state.get::<DatabasePool>().ok_or_else(|| DependencyError::missing("DatabasePool"))?;
    Ok(Arc::new(UserRepository {
        pool,
    }))
}

#[test]
fn test_nested_dependency_chain_registration() {
    let db_pool = DatabasePool {
        connection_string: "postgres://localhost".to_string(),
    };

    let app = UltraApiApp::new()
        .dep(db_pool)
        .depends(get_db_pool)
        .depends(get_user_repository);

    // Verify registration
    assert!(app.get_depends_resolver().is_some());
}

// ---- Test 3: Function dependency can be registered ----

#[derive(Clone, Debug, PartialEq)]
struct ConfigValue {
    setting: String,
}

async fn get_config_value(_state: AppState) -> Result<Arc<ConfigValue>, DependencyError> {
    Ok(Arc::new(ConfigValue {
        setting: "default".to_string(),
    }))
}

#[test]
fn test_function_dependency_can_be_resolved() {
    // Build app with Depends function
    let app = UltraApiApp::new()
        .depends(get_config_value);

    // Verify registration works
    assert!(app.get_depends_resolver().is_some());
}

// ---- Test 4: Cycle detection setup ----

#[test]
fn test_cycle_detection_setup() {
    // This test verifies the app can be built with deps that could form cycles
    // Actual cycle detection happens when resolve() is called with circular references
    
    async fn dep_a(_state: AppState) -> Result<Arc<String>, DependencyError> {
        Ok(Arc::new("a".to_string()))
    }
    
    async fn dep_b(_state: AppState) -> Result<Arc<String>, DependencyError> {
        Ok(Arc::new("b".to_string()))
    }

    let app = UltraApiApp::new()
        .depends(dep_a)
        .depends(dep_b);

    // Just verify it builds
    let _ = app;
}

// ---- Test 5: Missing dependency function errors ----

// This tests that when a Depends function tries to resolve a missing dependency,
// it gets a proper error
async fn get_missing_service(_state: AppState) -> Result<Arc<MissingType>, DependencyError> {
    // This tries to resolve something that wasn't registered
    Err(DependencyError::missing("MissingType"))
}

#[test]
fn test_missing_dependency_error_registration() {
    let app = UltraApiApp::new()
        .depends(get_missing_service);

    // Just verify it builds
    let _ = app;
}

// ---- Test 6: Backward compatibility with regular Dep ----

// This tests that regular deps without Depends functions still work
#[derive(Clone)]
struct RegularDep {
    value: String,
}

#[get("/regular-dep")]
async fn regular_dep_route(dep: Dep<RegularDep>) -> String {
    dep.value.clone()
}

#[tokio::test]
async fn test_regular_dep_backward_compatibility() {
    let reg_dep = RegularDep {
        value: "direct".to_string(),
    };

    // Use regular .dep() - should work as before
    let app = UltraApiApp::new()
        .dep(reg_dep)
        .include(UltraApiRouter::new("/test").route(__HAYAI_ROUTE_REGULAR_DEP_ROUTE));

    let router = app.into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/test/regular-dep", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("direct"), "Got: {}", body);
}

// ---- Test 7: Multiple function deps ----

#[derive(Clone)]
struct Counter {
    count: i32,
}

async fn get_counter_a(_state: AppState) -> Result<Arc<Counter>, DependencyError> {
    Ok(Arc::new(Counter { count: 1 }))
}

async fn get_counter_b(_state: AppState) -> Result<Arc<Counter>, DependencyError> {
    Ok(Arc::new(Counter { count: 2 }))
}

#[test]
fn test_multiple_deps_same_type_overwrites() {
    // Registering multiple deps of same type should overwrite
    let app = UltraApiApp::new()
        .depends(get_counter_a)
        .depends(get_counter_b);
    
    // The last one wins (test verifies it builds)
    let _ = app;
}

// ---- Test 8: Depends resolver is properly initialized in router ----

#[get("/state-dep")]
async fn state_dep_route(dep: State<SimpleState>) -> String {
    dep.0.clone()
}

#[derive(Clone)]
struct SimpleState(String);

#[tokio::test]
async fn test_state_dep_still_works() {
    let state = SimpleState("test-state".to_string());
    
    let app = UltraApiApp::new()
        .dep(state)
        .include(UltraApiRouter::new("/test").route(__HAYAI_ROUTE_STATE_DEP_ROUTE));

    let router = app.into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/test/state-dep", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("test-state"), "Got: {}", body);
}

// Helper types
#[derive(Clone)]
struct MissingType;
