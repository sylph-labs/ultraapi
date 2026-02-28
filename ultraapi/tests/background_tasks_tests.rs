//! BackgroundTasks 結合テスト
//!
//! "レスポンスが返った後に task が走る" ことを確認するテスト
//!
//! 注: 現時点ではUltraApiAppとの完全な統合はmacroの制限により直接テストできませんが、
//! response_tasks.rsのユニットテストで基本的な動作は検証済みです。

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
}
