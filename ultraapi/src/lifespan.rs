// Lifespan module for startup/shutdown hooks
use std::sync::Arc;

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
#[derive(Clone)]
pub struct Lifecycle {
    startup_hooks: Vec<StartupHook>,
    shutdown_hooks: Vec<ShutdownHook>,
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new()
    }
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
}

/// Re-export AppState for use in lifecycle hooks
use crate::AppState;
