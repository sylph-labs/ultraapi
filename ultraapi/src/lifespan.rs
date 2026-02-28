// Lifespan module for startup/shutdown hooks (ASGI Lifespan compatible concept)
//
// This module provides:
// - Lifecycle: startup/shutdown hook registration
// - LifespanRunner: a handle that guarantees hooks run consistently
//   across `serve()`, `TestClient`, and embedded usage.
//
// Design goals:
// - Startup runs at most once (even with concurrent requests)
// - Shutdown runs at most once
// - `into_router_with_lifespan()` returns a runner that can be used to
//   trigger shutdown in tests.

use std::sync::Arc;

use axum::{extract::Request, middleware::Next, Router};
use tokio::sync::{Notify, OnceCell};

use crate::AppState;

/// Lifecycle hook type - async function that runs on startup
pub type StartupHook = Arc<
    dyn Fn(&AppState) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync>>
        + Send
        + Sync,
>;

/// Lifecycle hook type - async function that runs on shutdown
pub type ShutdownHook = Arc<
    dyn Fn(&AppState) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync>>
        + Send
        + Sync,
>;

/// Application lifecycle manager for startup and shutdown hooks
#[derive(Clone, Default)]
pub struct Lifecycle {
    startup_hooks: Vec<StartupHook>,
    shutdown_hooks: Vec<ShutdownHook>,
}

impl Lifecycle {
    /// Create a new Lifecycle manager
    pub fn new() -> Self {
        Self {
            startup_hooks: Vec::new(),
            shutdown_hooks: Vec::new(),
        }
    }

    /// Add a startup hook
    ///
    /// # Example
    /// ```
    /// use ultraapi::prelude::Lifecycle;
    ///
    /// let _lifecycle = Lifecycle::new().on_startup(|_state| {
    ///     Box::pin(async move {
    ///         println!("Application starting up!");
    ///     })
    /// });
    /// ```
    pub fn on_startup<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(&AppState) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + Sync + 'static,
    {
        self.startup_hooks.push(Arc::new(move |state| {
            Box::pin(hook(state))
                as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync>>
        }));
        self
    }

    /// Add a shutdown hook
    ///
    /// # Example
    /// ```
    /// use ultraapi::prelude::Lifecycle;
    ///
    /// let _lifecycle = Lifecycle::new().on_shutdown(|_state| {
    ///     Box::pin(async move {
    ///         println!("Application shutting down!");
    ///     })
    /// });
    /// ```
    pub fn on_shutdown<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(&AppState) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + Sync + 'static,
    {
        self.shutdown_hooks.push(Arc::new(move |state| {
            Box::pin(hook(state))
                as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync>>
        }));
        self
    }

    /// Run all startup hooks
    pub async fn run_startup(&self, state: &AppState) {
        for hook in &self.startup_hooks {
            hook(state).await;
        }
    }

    /// Run all shutdown hooks
    pub async fn run_shutdown(&self, state: &AppState) {
        for hook in &self.shutdown_hooks {
            hook(state).await;
        }
    }

    /// Check if there are any startup hooks registered
    pub fn has_startup_hooks(&self) -> bool {
        !self.startup_hooks.is_empty()
    }

    /// Check if there are any shutdown hooks registered
    pub fn has_shutdown_hooks(&self) -> bool {
        !self.shutdown_hooks.is_empty()
    }
}

struct LifespanRunnerInner {
    lifecycle: Lifecycle,
    state: AppState,
    startup_once: OnceCell<()>,
    shutdown_once: OnceCell<()>,
    shutdown_notify: Arc<Notify>,
}

/// Lifespan runner that manages lifecycle execution.
///
/// - `ensure_startup()` runs startup hooks at most once
/// - `shutdown()` runs shutdown hooks at most once and signals waiters
/// - `wait_for_shutdown()` resolves once shutdown was triggered
#[derive(Clone)]
pub struct LifespanRunner {
    inner: Arc<LifespanRunnerInner>,
}

impl LifespanRunner {
    /// Create a new LifespanRunner.
    pub fn new(lifecycle: Lifecycle, state: AppState) -> Self {
        Self {
            inner: Arc::new(LifespanRunnerInner {
                lifecycle,
                state,
                startup_once: OnceCell::new(),
                shutdown_once: OnceCell::new(),
                shutdown_notify: Arc::new(Notify::new()),
            }),
        }
    }

    /// Get the AppState used by this runner.
    pub fn state(&self) -> &AppState {
        &self.inner.state
    }

    /// Ensure startup hooks have executed.
    ///
    /// This is safe to call multiple times; hooks will run only once.
    pub async fn ensure_startup(&self) {
        let lifecycle = self.inner.lifecycle.clone();
        let state = self.inner.state.clone();
        self.inner
            .startup_once
            .get_or_init(|| async move {
                lifecycle.run_startup(&state).await;
            })
            .await;
    }

    /// Trigger shutdown hooks (at most once) and notify waiters.
    pub async fn shutdown(&self) {
        // If the app never served a request, users still expect shutdown to be safe.
        // We don't forcibly run startup here, but we do allow shutdown to run.
        let lifecycle = self.inner.lifecycle.clone();
        let state = self.inner.state.clone();
        let notify = self.inner.shutdown_notify.clone();

        self.inner
            .shutdown_once
            .get_or_init(|| async move {
                lifecycle.run_shutdown(&state).await;
                notify.notify_waiters();
            })
            .await;
    }

    /// Wait for shutdown to be triggered.
    ///
    /// This is mainly used by `TestClient` / servers to implement graceful shutdown.
    pub async fn wait_for_shutdown(&self) {
        if self.inner.shutdown_once.get().is_some() {
            return;
        }
        self.inner.shutdown_notify.notified().await;
    }

    /// Convert to a router layer that ensures startup is executed lazily
    /// on the first request.
    pub fn into_layer(self) -> impl Fn(Router) -> Router + Clone + Send + Sync {
        let runner = Arc::new(self);

        move |router: Router| {
            let runner_clone = runner.clone();

            router.layer(axum::middleware::from_fn(
                move |request: Request, next: Next| {
                    let runner = runner_clone.clone();
                    async move {
                        runner.ensure_startup().await;
                        next.run(request).await
                    }
                },
            ))
        }
    }
}

/// Extension trait for adding lifespan to Router
pub trait RouterExt {
    /// Add lifespan management to a router
    fn with_lifespan(self, runner: LifespanRunner) -> Self;
}

impl RouterExt for Router {
    fn with_lifespan(self, runner: LifespanRunner) -> Self {
        let runner = Arc::new(runner);
        let runner_clone = runner.clone();

        self.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let runner = runner_clone.clone();
                async move {
                    runner.ensure_startup().await;
                    next.run(request).await
                }
            },
        ))
    }
}
