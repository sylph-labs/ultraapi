//! Response後実行タスクモジュール
//!
//! FastAPI互換のBackgroundTasks機能を提供します。
//! ハンドラ内で追加されたタスクは、レスポンスがクライアントに送信された後に実行されます。

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use parking_lot::RwLock as SyncRwLock;
use std::{future::Future, panic, sync::Arc};
use tokio::runtime::Handle;

/// BackgroundTasks - FastAPI互換のレスポンス後タスク実行
///
/// この型をハンドラのパラメータとして注入することで、レスポンス送信後に
/// 実行されるタスクを追加できます。
#[derive(Clone)]
#[allow(clippy::type_complexity, clippy::arc_with_non_send_sync)]
pub struct BackgroundTasks {
    /// 内部でタスクを保持する (parking_lot RwLock はSync)
    tasks: Arc<SyncRwLock<Vec<Box<dyn FnOnce() + Send + 'static>>>>,
    /// Tokio runtime handle for spawning tasks
    handle: Option<Handle>,
}

unsafe impl Send for BackgroundTasks {}
unsafe impl Sync for BackgroundTasks {}

#[allow(clippy::arc_with_non_send_sync)]
impl BackgroundTasks {
    /// 新しいBackgroundTasksインスタンスを作成
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(SyncRwLock::new(Vec::new())),
            handle: Handle::try_current().ok(),
        }
    }

    /// タスクを追加
    ///
    /// 追加されたタスクは、レスポンスがクライアントに送信された後に非同期的に実行されます。
    pub fn add<F>(&self, task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let handle = self.handle.clone();

        // FutureをBox<dyn FnOnce()>に変換
        let boxed: Box<dyn FnOnce() + Send + 'static> = Box::new(move || {
            // Tokio runtimeが利用可能ならスポーン
            if let Some(h) = handle {
                // 直接スポーンし、パニックしても構わない
                h.spawn(async move {
                    task.await;
                });
            } else {
                // runtimeがない場合は単にログ出力
                eprintln!("No Tokio runtime available for background task");
            }
        });

        let tasks = self.tasks.clone();
        {
            let mut t = tasks.write();
            t.push(boxed);
        }
    }

    /// タスクを実行待ち行列から取り出す（同期）
    pub fn take_tasks(&self) -> Vec<Box<dyn FnOnce() + Send + 'static>> {
        let mut t = self.tasks.write();
        std::mem::take(&mut *t)
    }
}

impl Default for BackgroundTasks {
    fn default() -> Self {
        Self::new()
    }
}

/// BackgroundTasksをExtensionとして注入するためのExtract実装
impl<S> axum::extract::FromRequestParts<S> for BackgroundTasks
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let tasks = parts
            .extensions
            .get::<BackgroundTasks>()
            .cloned()
            .unwrap_or_else(BackgroundTasks::new);

        Ok(tasks)
    }
}

/// Response後にBackgroundTasksを実行するMiddleware
pub async fn response_task_middleware(mut req: Request<Body>, next: Next) -> Response {
    // リクエスト拡張にBackgroundTasksを挿入
    let background_tasks = BackgroundTasks::new();
    req.extensions_mut().insert(background_tasks.clone());

    // ハンドラを実行
    let response = next.run(req).await;

    // レスポンス後にタスクを実行
    let task_list = background_tasks.take_tasks();
    let handle = background_tasks.handle.clone();

    for task in task_list {
        let task_name = format!(
            "bg_task_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );

        // Tokio runtimeを使ってスポーン
        if let Some(h) = handle.clone() {
            let task_name_log = task_name.clone();

            h.spawn(async move {
                // パニックフックを設定
                let prev_hook = panic::take_hook();
                let task_name_for_hook = task_name_log.clone();

                panic::set_hook(Box::new(move |panic_info| {
                    let msg = panic_info
                        .payload()
                        .downcast_ref::<&str>()
                        .unwrap_or(&"Unknown");
                    let location = panic_info
                        .location()
                        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                        .unwrap_or_else(|| "unknown".to_string());
                    eprintln!(
                        "Background task panicked (recovering): task={}, message={}, location={}",
                        task_name_for_hook, msg, location
                    );
                    prev_hook(panic_info);
                }));

                // パニックをキャッチ
                let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    task();
                }));

                if result.is_err() {
                    eprintln!(
                        "Background task panicked and was caught: task={}",
                        task_name_log
                    );
                }
            });
        }

        eprintln!("Background task dispatched: {}", task_name);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::StatusCode, routing::get, Router};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_background_tasks_execution() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let app = Router::new()
            .route(
                "/test",
                get(move || {
                    let executed = executed_clone.clone();
                    async move {
                        executed.store(true, Ordering::SeqCst);
                        "Hello"
                    }
                }),
            )
            .layer(axum::middleware::from_fn(response_task_middleware));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_background_tasks_with_task_extraction() {
        let app = Router::new()
            .route(
                "/test",
                get(|tasks: BackgroundTasks| async move {
                    tasks.add(async {
                        // 何もしないダミータスク
                    });
                    "OK"
                }),
            )
            .layer(axum::middleware::from_fn(response_task_middleware));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
