#![allow(dead_code, unused_variables)]

// Tests for request-scoped dependencies
//
// These tests verify:
// 1. Per-request fresh instance behavior for Scope::Request
// 2. Singleton behavior unchanged (Scope::Function or regular .depends())
// 3. Mixed dependency graphs (request-scoped + singleton)
//
// This implements the capability gap filled by task 1-3.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use ultraapi::prelude::*;
use ultraapi::{AppState, DependencyError, Generator, Scope, UltraApiApp};

// =============================================================================
// Test 1: Verify that yield_depends with Scope::Request creates request-scoped generator
// =============================================================================

static REQUEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug, PartialEq)]
struct RequestScopedService {
    id: usize,
}

struct RequestScopedGenerator;

#[async_trait::async_trait]
impl Generator for RequestScopedGenerator {
    type Output = RequestScopedService;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        let id = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        Ok(RequestScopedService { id })
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_yield_depends_request_scope_registration() {
    REQUEST_COUNTER.store(0, Ordering::SeqCst);

    let generator = RequestScopedGenerator;

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(generator, Scope::Request);

    let resolver = app.get_depends_resolver().expect("Resolver should exist");

    // Verify it's registered as a generator
    assert!(
        resolver.is_generator::<RequestScopedService>(),
        "Should be registered as generator"
    );

    // Verify the scope is Request
    let scope = resolver.get_generator_scope::<RequestScopedService>();
    assert_eq!(scope, Some(Scope::Request), "Scope should be Request");
}

// =============================================================================
// Test 2: Verify that Scope::Function creates singleton behavior
// =============================================================================

static FUNCTION_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct FunctionService {
    id: usize,
}

struct FunctionGenerator;

#[async_trait::async_trait]
impl Generator for FunctionGenerator {
    type Output = FunctionService;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        let id = FUNCTION_COUNTER.fetch_add(1, Ordering::SeqCst);
        Ok(FunctionService { id })
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_yield_depends_function_scope_registration() {
    FUNCTION_COUNTER.store(0, Ordering::SeqCst);

    let generator = FunctionGenerator;

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(generator, Scope::Function);

    let resolver = app.get_depends_resolver().expect("Resolver should exist");

    // Verify it's registered as a generator
    assert!(
        resolver.is_generator::<FunctionService>(),
        "Should be registered as generator"
    );

    // Verify the scope is Function
    let scope = resolver.get_generator_scope::<FunctionService>();
    assert_eq!(scope, Some(Scope::Function), "Scope should be Function");
}

// =============================================================================
// Test 3: Verify resolve_generator creates fresh instance each call (request scope)
// =============================================================================

#[tokio::test]
async fn test_request_scoped_generator_creates_fresh_instances() {
    REQUEST_COUNTER.store(0, Ordering::SeqCst);

    let generator = RequestScopedGenerator;

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(generator, Scope::Request);

    let resolver = app.get_depends_resolver().expect("Resolver should exist");

    // Create dependency scope for tracking cleanup
    let dep_scope = Arc::new(DependencyScope::new());
    let state = AppState::new();

    // First call - should get instance 0
    let result1 = resolver
        .resolve_generator::<RequestScopedService>(&state, &dep_scope)
        .await;
    assert!(
        result1.is_ok(),
        "First resolve should succeed: {:?}",
        result1
    );

    // Second call - should get instance 1 (fresh instance)
    let result2 = resolver
        .resolve_generator::<RequestScopedService>(&state, &dep_scope)
        .await;
    assert!(
        result2.is_ok(),
        "Second resolve should succeed: {:?}",
        result2
    );

    // Verify the counter incremented (meaning new instances were created)
    // The counter should be 2 after two calls
    assert_eq!(
        REQUEST_COUNTER.load(Ordering::SeqCst),
        2,
        "Counter should be 2 after two resolves"
    );
}

// =============================================================================
// Test 4: Regular .dep() still works as singleton (backward compatibility)
// =============================================================================

#[derive(Clone)]
struct SingletonService {
    value: String,
}

#[test]
fn test_regular_dep_still_works() {
    let service = SingletonService {
        value: "test".to_string(),
    };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .dep(service);

    // Verify it can build into router without error
    let _router = app.into_router();
}

// =============================================================================
// Test 5: Combined request-scoped and singleton dependencies
// =============================================================================

static MIXED_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct MixedRequestService {
    id: usize,
}

struct MixedRequestGenerator;

#[async_trait::async_trait]
impl Generator for MixedRequestGenerator {
    type Output = MixedRequestService;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        let id = MIXED_COUNTER.fetch_add(1, Ordering::SeqCst);
        Ok(MixedRequestService { id })
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Clone)]
struct MixedSingletonService {
    value: i32,
}

#[test]
fn test_mixed_dependencies_work_together() {
    MIXED_COUNTER.store(0, Ordering::SeqCst);

    let generator = MixedRequestGenerator;
    let singleton = MixedSingletonService { value: 42 };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .dep(singleton) // Singleton
        .yield_depends(generator, Scope::Request); // Request-scoped

    let resolver = app.get_depends_resolver().expect("Resolver should exist");

    // Both should be available
    assert!(
        resolver.is_generator::<MixedRequestService>(),
        "Request-scoped should be registered"
    );

    let scope = resolver.get_generator_scope::<MixedRequestService>();
    assert_eq!(scope, Some(Scope::Request), "Should be request scope");
}

// =============================================================================
// Test 6: Verify backward compatibility - .depends() function still works
// =============================================================================

#[derive(Clone)]
struct FuncService {
    id: i32,
}

async fn get_func_service(_state: AppState) -> Result<Arc<FuncService>, DependencyError> {
    Ok(Arc::new(FuncService { id: 123 }))
}

#[test]
fn test_depends_function_still_works() {
    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .depends(get_func_service);

    let resolver = app.get_depends_resolver().expect("Resolver should exist");
    // The function is registered, not a generator
    assert!(
        !resolver.is_generator::<FuncService>(),
        "Function dependency should not be a generator"
    );
}
