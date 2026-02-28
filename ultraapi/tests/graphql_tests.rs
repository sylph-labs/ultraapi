// Tests for GraphQL support

#[cfg(feature = "graphql")]
mod tests {
    use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
    use axum::{
        body::Body,
        extract::{Json, State},
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::post,
        Router,
    };
    use tower::ServiceExt;

    use ultraapi::graphql::{graphiql, graphql_post_handler};

    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        async fn hello(&self) -> &str {
            "Hello"
        }
    }

    type MySchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

    async fn graphql_endpoint(
        State(schema): State<MySchema>,
        Json(req): Json<async_graphql::Request>,
    ) -> impl IntoResponse {
        graphql_post_handler(&schema, req).await
    }

    #[tokio::test]
    async fn test_graphiql_endpoint_html() {
        let response = graphiql("/graphql", None).await.into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(response_str.contains("graphiql"));
        assert!(response_str.contains("/graphql"));
    }

    #[tokio::test]
    async fn test_graphql_query_success() {
        let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish();

        let app = Router::new()
            .route("/graphql", post(graphql_endpoint))
            .with_state(schema);

        let req_body = serde_json::json!({ "query": "{ hello }" }).to_string();

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/graphql")
                    .header("content-type", "application/json")
                    .body(Body::from(req_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(v["data"]["hello"], "Hello");
    }
}
