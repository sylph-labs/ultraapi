//! BackgroundTasks 結合テスト
//!
//! "レスポンスが返った後に task が走る" ことを確認するテスト
//! - 素の axum Router での動作
//! - UltraApiApp + #[get] ルート経路での E2E 動作

#[cfg(test)]
mod integration {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;
    use ultraapi::prelude::*;
    use ultraapi::response_tasks::{response_task_middleware, BackgroundTasks};

    /// レスポンス送信後にバックグラウンドタスクが実行されることを確認
    #[tokio::test]
    async fn test_task_runs_after_response() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let app = Router::new()
            .route(
                "/test",
                get(move |tasks: BackgroundTasks| {
                    let executed = executed_clone.clone();
                    async move {
                        // バックグラウンドタスクを追加
                        tasks.add(async move {
                            // 少し待って確実にレスポンス送信後に実行
                            tokio::time::sleep(Duration::from_millis(10)).await;
                            executed.store(true, Ordering::SeqCst);
                        });

                        "Response sent"
                    }
                }),
            )
            .layer(axum::middleware::from_fn(response_task_middleware));

        // リクエストを実行
        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // レスポンスが返った後、タスクが実行されるのを少し待つ
        tokio::time::sleep(Duration::from_millis(50)).await;

        // タスクが実行されたことを確認
        assert!(
            executed.load(Ordering::SeqCst),
            "Background task should have run after response"
        );
    }

    /// 複数のタスクが正しく実行されることを確認
    #[tokio::test]
    async fn test_multiple_tasks() {
        let counter = Arc::new(AtomicBool::new(false));
        let counter_clone = counter.clone();

        let app = Router::new()
            .route(
                "/test",
                get(move |tasks: BackgroundTasks| {
                    let counter = counter_clone.clone();
                    async move {
                        // 複数のタスクを追加
                        for i in 0..3 {
                            let c = counter.clone();
                            tasks.add(async move {
                                tokio::time::sleep(Duration::from_millis(10 * (i + 1))).await;
                                // 最後のタスクでのみフラグを立てる
                                if i == 2 {
                                    c.store(true, Ordering::SeqCst);
                                }
                            });
                        }

                        "Response sent"
                    }
                }),
            )
            .layer(axum::middleware::from_fn(response_task_middleware));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // 全てのタスクが完了するのを待つ
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(
            counter.load(Ordering::SeqCst),
            "All background tasks should have run"
        );
    }

    /// タスクを追加しない場合も正常に動作することを確認
    #[tokio::test]
    async fn test_no_tasks() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(axum::middleware::from_fn(response_task_middleware));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[derive(Clone)]
    struct BackgroundTaskFlag(Arc<AtomicBool>);

    #[get("/e2e/background-tasks")]
    async fn background_tasks_e2e_route(
        tasks: BackgroundTasks,
        flag: Dep<BackgroundTaskFlag>,
    ) -> &'static str {
        let executed = flag.0.clone();
        tasks.add(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            executed.store(true, Ordering::SeqCst);
        });
        "queued"
    }

    /// UltraApiApp + #[get] ルートでもレスポンス後にタスクが実行されることを確認
    #[tokio::test]
    async fn test_ultraapi_app_route_runs_background_tasks_after_response() {
        let executed = Arc::new(AtomicBool::new(false));

        let app = UltraApiApp::new()
            .dep(BackgroundTaskFlag(executed.clone()))
            .include(UltraApiRouter::new("").route(__HAYAI_ROUTE_BACKGROUND_TASKS_E2E_ROUTE));

        let response = app
            .into_router()
            .oneshot(
                Request::builder()
                    .uri("/e2e/background-tasks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(
            executed.load(Ordering::SeqCst),
            "Background task should have run after response via UltraApiApp route"
        );
    }
}
