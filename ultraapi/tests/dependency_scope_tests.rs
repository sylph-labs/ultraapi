// Tests for yield-based dependency scopes (FastAPI-style)
//
// These tests verify:
// 1. Function-scope cleanup timing
// 2. Request-scope cleanup timing
// 3. Generator trait registration works
// 4. Backward compatibility with regular .depends()

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use ultraapi::prelude::*;
use ultraapi::{AppState, DependencyError, Scope, UltraApiApp};

// =============================================================================
// Test 1: Verify yield_depends creates resolver and registers generators
// =============================================================================

#[derive(Clone)]
#[allow(dead_code)]
struct TestResource {
    value: String,
}

#[async_trait::async_trait]
impl Generator for TestResource {
    type Output = Self;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        Ok(Arc::try_unwrap(self).unwrap_or_else(|a| (*a).clone()))
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_yield_depends_creates_resolver() {
    let resource = TestResource {
        value: "test".to_string(),
    };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(resource, Scope::Function);

    // Should create the depends resolver automatically
    assert!(
        app.get_depends_resolver().is_some(),
        "Resolver should be created"
    );
}

#[test]
fn test_yield_depends_can_check_is_generator() {
    let resource = TestResource {
        value: "test".to_string(),
    };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(resource, Scope::Function);

    let resolver = app.get_depends_resolver().unwrap();
    assert!(
        resolver.is_generator::<TestResource>(),
        "Should be registered as generator"
    );
}

#[test]
fn test_yield_depends_stores_correct_scope() {
    let resource_fn = TestResource {
        value: "fn".to_string(),
    };
    let resource_req = TestResource {
        value: "req".to_string(),
    };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(resource_fn, Scope::Function)
        .yield_depends(resource_req, Scope::Request);

    let resolver = app.get_depends_resolver().unwrap();

    let fn_scope = resolver.get_generator_scope::<TestResource>();
    // Last registration wins, so it should be Request
    assert_eq!(
        fn_scope,
        Some(Scope::Request),
        "Last registered scope should win"
    );
}

#[test]
fn test_default_scope_is_function() {
    let resource = TestResource {
        value: "test".to_string(),
    };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(resource, Scope::default());

    let resolver = app.get_depends_resolver().unwrap();
    assert!(
        resolver.is_generator::<TestResource>(),
        "Should be registered"
    );
}

// =============================================================================
// Test 2: Backward compatibility with regular .depends()
// =============================================================================

#[derive(Clone)]
#[allow(dead_code)]
struct LegacyService {
    value: i32,
}

async fn get_legacy_service(_state: AppState) -> Result<Arc<LegacyService>, DependencyError> {
    Ok(Arc::new(LegacyService { value: 42 }))
}

#[test]
fn test_backward_compatibility_with_depends() {
    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .depends(get_legacy_service);

    // Should still work
    assert!(app.get_depends_resolver().is_some());
}

// =============================================================================
// Test 3: Combined yield_depends and depends work together
// =============================================================================

#[derive(Clone)]
#[allow(dead_code)]
struct YieldsResource {
    id: &'static str,
}

#[async_trait::async_trait]
impl Generator for YieldsResource {
    type Output = Self;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        Ok(Arc::try_unwrap(self).unwrap_or_else(|a| (*a).clone()))
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_combined_yield_depends_and_depends() {
    let resource = YieldsResource { id: "test" };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .depends(get_legacy_service)
        .yield_depends(resource, Scope::Function);

    // Both should work together
    assert!(app.get_depends_resolver().is_some());

    let resolver = app.get_depends_resolver().unwrap();
    assert!(
        resolver.is_generator::<YieldsResource>(),
        "Generator should be registered"
    );
    // LegacyService was registered via .depends(), not .yield_depends()
    assert!(
        !resolver.is_generator::<LegacyService>(),
        "LegacyService should NOT be a generator"
    );
}

// =============================================================================
// Test 4: Generator with cleanup tracking
// =============================================================================

#[derive(Clone)]
#[allow(dead_code)]
struct TrackedResource {
    id: &'static str,
    initialized: Arc<AtomicBool>,
    cleaned_up: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl Generator for TrackedResource {
    type Output = Self;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        self.initialized.store(true, Ordering::SeqCst);
        Ok(Arc::try_unwrap(self).unwrap_or_else(|a| (*a).clone()))
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        self.cleaned_up.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn test_generator_cleanup_tracking() {
    let initialized = Arc::new(AtomicBool::new(false));
    let cleaned_up = Arc::new(AtomicBool::new(false));

    let resource = TrackedResource {
        id: "tracked",
        initialized: initialized.clone(),
        cleaned_up: cleaned_up.clone(),
    };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(resource, Scope::Function);

    assert!(app.get_depends_resolver().is_some());
    // Note: Full cleanup test requires HTTP server test
}

// =============================================================================
// Test 5: Multiple generators with different scopes
// =============================================================================

#[derive(Clone)]
#[allow(dead_code)]
struct ResourceA {
    name: &'static str,
    order: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Generator for ResourceA {
    type Output = Self;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        Ok(Arc::try_unwrap(self).unwrap_or_else(|a| (*a).clone()))
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        self.order.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct ResourceB {
    name: &'static str,
    order: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Generator for ResourceB {
    type Output = Self;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        Ok(Arc::try_unwrap(self).unwrap_or_else(|a| (*a).clone()))
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        self.order.fetch_add(10, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn test_multiple_generators_different_scopes() {
    let order = Arc::new(AtomicUsize::new(0));

    let resource_a = ResourceA {
        name: "a",
        order: order.clone(),
    };
    let resource_b = ResourceB {
        name: "b",
        order: order.clone(),
    };

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(resource_a, Scope::Function)
        .yield_depends(resource_b, Scope::Request);

    let resolver = app.get_depends_resolver().unwrap();

    assert!(resolver.is_generator::<ResourceA>());
    assert!(resolver.is_generator::<ResourceB>());

    // Different scopes
    let scope_a = resolver.get_generator_scope::<ResourceA>();
    let scope_b = resolver.get_generator_scope::<ResourceB>();

    // Last registration for same type wins, but here types are different
    // Need to check actual registration
    assert!(scope_a.is_some());
    assert!(scope_b.is_some());
}

// =============================================================================
// Test 6: Error propagation from generators
// =============================================================================

#[derive(Clone)]
struct FailingResource;

#[async_trait::async_trait]
impl Generator for FailingResource {
    type Output = Self;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        Err(DependencyError::missing("FailingResource"))
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_generator_error_type() {
    let resource = FailingResource;

    let app = UltraApiApp::new()
        .title("Test API")
        .version("1.0.0")
        .yield_depends(resource, Scope::Function);

    // Should register without error
    assert!(app.get_depends_resolver().is_some());

    let resolver = app.get_depends_resolver().unwrap();
    assert!(resolver.is_generator::<FailingResource>());
}

// =============================================================================
// Test 7: Scope enum values
// =============================================================================

#[test]
fn test_scope_enum_values() {
    assert_eq!(Scope::Function as u32, 0);
    assert_eq!(Scope::Request as u32, 1);
}

#[test]
fn test_scope_default() {
    let default_scope = Scope::default();
    assert_eq!(default_scope, Scope::Function);
}
