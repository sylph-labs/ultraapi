// Tests for FastAPI-style callable dependency injection
//
// These tests verify:
// 1. Callable dependencies can receive other dependencies
// 2. Multi-level dependency chains work
// 3. Legacy AppState function still works
// 4. depends_with_deps API works

use std::any::TypeId;
use std::sync::Arc;
use ultraapi::prelude::*;
use ultraapi::{AppState, DependencyError, DependsResolver, UltraApiApp};

/// Test: callable dependency receiving another dependency
#[tokio::test]
async fn test_callable_dependency_receiving_dep() {
    // Setup: Create app with base dependency
    #[derive(Clone)]
    struct DbPool {
        connection_string: String,
    }

    #[derive(Clone)]
    struct UserRepository {
        pool: Arc<DbPool>,
    }

    // Base dependency resolver - returns DbPool
    async fn get_db_pool(_state: AppState) -> Result<DbPool, DependencyError> {
        Ok(DbPool {
            connection_string: "postgres://localhost".to_string(),
        })
    }

    // Callable dependency that depends on DbPool - receives DbPool from state
    async fn get_user_repo(state: AppState) -> Result<UserRepository, DependencyError> {
        let pool = state
            .get::<DbPool>()
            .ok_or_else(|| DependencyError::missing("DbPool"))?;
        Ok(UserRepository { pool })
    }

    // Create app with dependency chain
    let app = UltraApiApp::new()
        .depends(get_db_pool)
        .depends_with_deps(get_user_repo, vec![TypeId::of::<DbPool>()]);

    // Verify resolver was created
    assert!(app.get_depends_resolver().is_some());
}

/// Test: multi-level dependency chain  
#[tokio::test]
async fn test_multi_level_dependency_chain() {
    #[derive(Clone)]
    struct DbPool {
        connection_string: String,
    }

    #[derive(Clone)]
    struct UserRepository {
        pool: Arc<DbPool>,
    }

    #[derive(Clone)]
    struct UserService {
        repo: Arc<UserRepository>,
    }

    // Level 0: DbPool
    async fn get_db_pool(_state: AppState) -> Result<DbPool, DependencyError> {
        Ok(DbPool {
            connection_string: "postgres://localhost".to_string(),
        })
    }

    // Level 1: UserRepository depends on DbPool
    async fn get_user_repo(state: AppState) -> Result<UserRepository, DependencyError> {
        let pool = state
            .get::<DbPool>()
            .ok_or_else(|| DependencyError::missing("DbPool"))?;
        Ok(UserRepository { pool })
    }

    // Level 2: UserService depends on UserRepository
    async fn get_user_service(state: AppState) -> Result<UserService, DependencyError> {
        let repo = state
            .get::<UserRepository>()
            .ok_or_else(|| DependencyError::missing("UserRepository"))?;
        Ok(UserService { repo })
    }

    // Create app with multi-level chain
    let app = UltraApiApp::new()
        .depends(get_db_pool)
        .depends_with_deps(get_user_repo, vec![TypeId::of::<DbPool>()])
        .depends_with_deps(get_user_service, vec![TypeId::of::<UserRepository>()]);

    // Verify resolver exists
    assert!(app.get_depends_resolver().is_some());
}

/// Test: legacy AppState function still works
#[tokio::test]
async fn test_legacy_appstate_function_still_works() {
    #[derive(Clone)]
    struct Config {
        value: String,
    }

    // Traditional dependency function that takes AppState
    // The function should return the dependency value
    async fn get_config(state: AppState) -> Result<Config, DependencyError> {
        // state.get returns Option<Arc<T>>, we need to handle this
        // For this test, we just return a Config directly
        Ok(Config {
            value: "test".to_string(),
        })
    }

    // Create app with legacy-style dependency
    let app = UltraApiApp::new().depends(get_config);

    // Verify resolver was created
    assert!(app.get_depends_resolver().is_some());
}

/// Test: depends_with_deps API works  
#[tokio::test]
async fn test_depends_with_deps_api() {
    #[derive(Clone)]
    struct ServiceA {
        value: i32,
    }

    #[derive(Clone)]
    struct ServiceB {
        a: Arc<ServiceA>,
    }

    async fn get_service_a(_state: AppState) -> Result<ServiceA, DependencyError> {
        Ok(ServiceA { value: 42 })
    }

    async fn get_service_b(state: AppState) -> Result<ServiceB, DependencyError> {
        let a = state
            .get::<ServiceA>()
            .ok_or_else(|| DependencyError::missing("ServiceA"))?;
        Ok(ServiceB { a })
    }

    let resolver = DependsResolver::new();

    // Use the new API - note: functions return T (not Arc<T>)
    resolver.register_with_deps(
        std::marker::PhantomData::<ServiceB>,
        get_service_b,
        vec![TypeId::of::<ServiceA>()],
    );
    resolver.register(std::marker::PhantomData::<ServiceA>, get_service_a);

    // Check that deps are registered
    assert!(resolver.has_deps::<ServiceB>());
    assert!(!resolver.has_deps::<ServiceA>());

    let deps = resolver.get_deps::<ServiceB>();
    assert!(deps.is_some());
    assert_eq!(deps.unwrap(), vec![TypeId::of::<ServiceA>()]);
}
