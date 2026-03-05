// Tests for FastAPI-style callable dependency injection
//
// These tests verify:
// 1. Callable dependencies can receive other dependencies
// 2. Multi-level dependency chains work
// 3. Legacy AppState function still works
// 4. depends_with_deps API works

use std::any::TypeId;
use std::sync::Arc;
use ultraapi::{AppState, DependencyError, Depends, DependsResolver, UltraApiApp};

/// Test: callable dependency receiving another dependency
#[tokio::test]
async fn test_callable_dependency_receiving_dep() {
    // Setup: Create app with base dependency
    #[derive(Clone)]
    #[allow(dead_code)]
    struct DbPool {
        connection_string: String,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    struct DbPool {
        connection_string: String,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
    struct UserRepository {
        pool: Arc<DbPool>,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    struct Config {
        value: String,
    }

    // Traditional dependency function that takes AppState
    // The function should return the dependency value
    async fn get_config(_state: AppState) -> Result<Config, DependencyError> {
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
    #[allow(dead_code)]
    struct ServiceA {
        value: i32,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
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

#[tokio::test]
async fn test_depends_with_deps_resolves_nested_chain_automatically() {
    #[derive(Clone)]
    struct DbPool {
        dsn: String,
    }

    #[derive(Clone)]
    struct UserRepo {
        pool: Arc<DbPool>,
    }

    #[derive(Clone)]
    struct UserService {
        repo: Arc<UserRepo>,
    }

    async fn get_db_pool(_state: AppState) -> Result<DbPool, DependencyError> {
        Ok(DbPool {
            dsn: "postgres://localhost/ultraapi".to_string(),
        })
    }

    async fn get_user_repo(state: AppState) -> Result<UserRepo, DependencyError> {
        let pool = state
            .get::<DbPool>()
            .ok_or_else(|| DependencyError::missing("DbPool"))?;
        Ok(UserRepo { pool })
    }

    async fn get_user_service(state: AppState) -> Result<UserService, DependencyError> {
        let repo = state
            .get::<UserRepo>()
            .ok_or_else(|| DependencyError::missing("UserRepo"))?;
        Ok(UserService { repo })
    }

    let resolver = DependsResolver::new();
    resolver.register(std::marker::PhantomData::<DbPool>, get_db_pool);
    resolver.register_with_deps(
        std::marker::PhantomData::<UserRepo>,
        get_user_repo,
        vec![TypeId::of::<DbPool>()],
    );
    resolver.register_with_deps(
        std::marker::PhantomData::<UserService>,
        get_user_service,
        vec![TypeId::of::<UserRepo>()],
    );

    let service = resolver
        .resolve::<UserService>(&AppState::new())
        .await
        .expect("UserService should resolve with nested chain");

    assert_eq!(service.repo.pool.dsn, "postgres://localhost/ultraapi");
}

#[tokio::test]
async fn test_depends_with_deps_missing_dependency_returns_clear_error() {
    #[derive(Clone)]
    struct MissingDep;

    #[derive(Clone, Debug)]
    struct ServiceWithMissingDep;

    async fn get_service_with_missing_dep(
        state: AppState,
    ) -> Result<ServiceWithMissingDep, DependencyError> {
        let _ = state
            .get::<MissingDep>()
            .ok_or_else(|| DependencyError::missing("MissingDep"))?;
        Ok(ServiceWithMissingDep)
    }

    let resolver = DependsResolver::new();
    resolver.register_with_deps(
        std::marker::PhantomData::<ServiceWithMissingDep>,
        get_service_with_missing_dep,
        vec![TypeId::of::<MissingDep>()],
    );

    let err = resolver
        .resolve::<ServiceWithMissingDep>(&AppState::new())
        .await
        .expect_err("missing declared dependency should error");

    let message = err.to_string();
    assert!(message.contains("Dependency not found in chain"));
    assert!(message.contains("ServiceWithMissingDep"));
}

#[tokio::test]
async fn test_depends_with_deps_cycle_returns_clear_error() {
    #[derive(Clone, Debug)]
    struct ServiceA;

    #[derive(Clone)]
    struct ServiceB;

    async fn get_service_a(state: AppState) -> Result<ServiceA, DependencyError> {
        let _ = state
            .get::<ServiceB>()
            .ok_or_else(|| DependencyError::missing("ServiceB"))?;
        Ok(ServiceA)
    }

    async fn get_service_b(state: AppState) -> Result<ServiceB, DependencyError> {
        let _ = state
            .get::<ServiceA>()
            .ok_or_else(|| DependencyError::missing("ServiceA"))?;
        Ok(ServiceB)
    }

    let resolver = DependsResolver::new();
    resolver.register_with_deps(
        std::marker::PhantomData::<ServiceA>,
        get_service_a,
        vec![TypeId::of::<ServiceB>()],
    );
    resolver.register_with_deps(
        std::marker::PhantomData::<ServiceB>,
        get_service_b,
        vec![TypeId::of::<ServiceA>()],
    );

    let err = resolver
        .resolve::<ServiceA>(&AppState::new())
        .await
        .expect_err("cycle should be detected");

    let message = err.to_string();
    assert!(message.contains("Circular dependency detected"));
    assert!(message.contains("ServiceA"));
    assert!(message.contains("ServiceB"));
}

#[tokio::test]
async fn test_depends_accepts_fastapi_style_callable_signature() {
    #[derive(Clone)]
    struct DbPool {
        dsn: String,
    }

    #[derive(Clone)]
    struct UserRepo {
        pool: Arc<DbPool>,
    }

    async fn get_db_pool() -> Result<DbPool, DependencyError> {
        Ok(DbPool {
            dsn: "postgres://localhost/ultraapi".to_string(),
        })
    }

    async fn get_user_repo(pool: Depends<DbPool>) -> Result<UserRepo, DependencyError> {
        Ok(UserRepo {
            pool: Arc::clone(&pool.0),
        })
    }

    let app = UltraApiApp::new()
        .depends(get_db_pool)
        .depends(get_user_repo);

    let resolver = app
        .get_depends_resolver()
        .expect("resolver should exist for callable dependencies");

    let repo = resolver
        .resolve::<UserRepo>(&AppState::new())
        .await
        .expect("callable dependency with Depends/State should resolve");

    assert_eq!(repo.pool.dsn, "postgres://localhost/ultraapi");
}
