//! Background task management module
use std::future::Future;
use std::panic;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub enum TaskKind { OneShot, Recurring }

#[derive(Clone)]
pub struct BackgroundTasks { sender: mpsc::Sender<TaskCommand> }

enum TaskCommand {
    Spawn { name: String, future: Box<dyn FnOnce() -> JoinHandle<()> + Send> },
    SpawnInterval { name: String, interval_secs: u64, future: Box<dyn Fn() -> JoinHandle<()> + Send> },
    Shutdown,
}

struct TaskEntry { name: String, handle: JoinHandle<()> }

impl BackgroundTasks {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<TaskCommand>(100);
        tokio::spawn(async move {
            let mut tasks: Vec<TaskEntry> = Vec::new();
            loop {
                let Some(cmd) = receiver.recv().await else { break; };
                match cmd {
                    TaskCommand::Spawn { name, future } => {
                        let handle = future();
                        let task_name = name.clone();
                        tasks.push(TaskEntry { name, handle });
                        info!(task.name = %task_name, "Background task spawned");
                    }
                    TaskCommand::SpawnInterval { name, interval_secs, future } => {
                        let task_name_for_info = name.clone();
                        let task_name_for_loop = name.clone();
                        let interval_for_loop = interval_secs;
                        let handle = tokio::spawn(async move {
                            loop {
                                let task_handle = (future)();
                                Self::run_with_panic_recovery(task_name_for_loop.clone(), interval_for_loop, task_handle).await;
                            }
                        });
                        tasks.push(TaskEntry { name, handle });
                        info!(task.name = %task_name_for_info, interval_secs = %interval_secs, "Recurring background task spawned");
                    }
                    TaskCommand::Shutdown => { info!("Shutdown command received"); break; }
                }
            }
            if !tasks.is_empty() {
                for task in tasks {
                    let name = task.name.clone();
                    match tokio::time::timeout(tokio::time::Duration::from_secs(5), task.handle).await {
                        Ok(Ok(())) => info!(task.name = %name, "Task completed"),
                        Ok(Err(e)) => if e.is_panic() { warn!(task.name = %name, "Task panicked") } else { warn!(task.name = %name, error = ?e, "Task failed") },
                        Err(_) => warn!(task.name = %name, "Task timed out"),
                    }
                }
            }
            info!("All background tasks shut down");
        });
        Self { sender }
    }

    async fn run_with_panic_recovery(task_name: String, interval_secs: u64, handle: JoinHandle<()>) {
        let task_name_for_panic = task_name.clone();
        let hook_for_panic = Box::new(move |panic_info: &panic::PanicHookInfo| {
            let msg = panic_info.payload().downcast_ref::<&str>().unwrap_or(&"Unknown").to_string();
            let location = panic_info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown".to_string());
            error!(task.name = %task_name_for_panic, panic.message = %msg, panic.location = %location, "Background task panicked (will restart)");
        });
        panic::set_hook(hook_for_panic);
        match handle.await {
            Ok(()) => info!(task.name = %task_name, "Background task completed"),
            Err(e) => if e.is_panic() { error!(task.name = %task_name, "Background task panicked") } else { warn!(task.name = %task_name, error = ?e, "Background task failed") },
        }
    }

    pub fn spawn<F, Fut>(&self, name: impl Into<String>, task: F) where F: Future<Output = ()> + Send + 'static {
        let name = name.into();
        let task_name = name.clone();
        let future = Box::new(move || {
            tokio::spawn(async move {
                let task_name_for_panic = task_name.clone();
                let hook = Box::new(move |panic_info: &panic::PanicHookInfo| {
                    let msg = panic_info.payload().downcast_ref::<&str>().unwrap_or(&"Unknown").to_string();
                    let location = panic_info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown".to_string());
                    error!(task.name = %task_name_for_panic, panic.message = %msg, panic.location = %location, "Background task panicked");
                });
                panic::set_hook(hook);
                task.await;
                info!(task.name = %task_name, "Background task completed successfully");
            })
        });
        let _ = self.sender.try_send(TaskCommand::Spawn { name, future });
    }

    pub fn spawn_interval<F, Fut>(&self, name: impl Into<String>, interval: tokio::time::Duration, task: F) where F: Future<Output = ()> + Send + Clone + 'static {
        let name = name.into();
        let interval_secs = interval.as_secs();
        let task_clone = task.clone();
        let future = Box::new(move || {
            let task = task_clone.clone();
            tokio::spawn(async move {
                let task_name_for_panic = name.clone();
                let hook = Box::new(move |panic_info: &panic::PanicHookInfo| {
                    let msg = panic_info.payload().downcast_ref::<&str>().unwrap_or(&"Unknown").to_string();
                    let location = panic_info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown".to_string());
                    error!(task.name = %task_name_for_panic, panic.message = %msg, panic.location = %location, "Recurring background task panicked (will restart)");
                });
                panic::set_hook(hook);
                task.await;
            })
        });
        let _ = self.sender.try_send(TaskCommand::SpawnInterval { name, interval_secs, future });
    }

    pub async fn shutdown(&self) { info!("Initiating graceful shutdown"); let _ = self.sender.send(TaskCommand::Shutdown).await; }
}

impl Default for BackgroundTasks { fn default() -> Self { Self::new() } }

pub trait WithBackgroundTasks { fn with_background_tasks(self) -> Self; }
impl WithBackgroundTasks for crate::HayaiApp { fn with_background_tasks(self) -> Self { self } }

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber;
    #[tokio::test]
    async fn test_panic_recovery() {
        let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
        let tasks = BackgroundTasks::new();
        tasks.spawn("panic_task", async { panic!("This is a test panic") });
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let result = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let result_clone = result.clone();
        tasks.spawn("healthy_task", async move { result_clone.store(true, std::sync::atomic::Ordering::SeqCst); });
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(result.load(std::sync::atomic::Ordering::SeqCst), "Server should remain operational");
        info!("Test passed");
    }
    #[tokio::test]
    async fn test_graceful_shutdown() {
        let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
        let tasks = BackgroundTasks::new();
        let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let completed_clone = completed.clone();
        tasks.spawn("shutdown_test", async move { tokio::time::sleep(tokio::time::Duration::from_millis(50)).await; completed_clone.store(true, std::sync::atomic::Ordering::SeqCst); });
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        tasks.shutdown().await;
        assert!(completed.load(std::sync::atomic::Ordering::SeqCst), "Task should complete during shutdown");
        info!("Test passed");
    }
}
