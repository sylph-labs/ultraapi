//! GraphQL support for UltraAPI
//!
//! This module provides GraphQL endpoint handlers using async-graphql.
//! It includes support for GraphQL queries via POST and interactive GraphiQL/Playground via GET.
//!
//! # Example
//!
//! ```ignore
//! use async_graphql::SimpleObject;
//! use async_graphql::Schema;
//! use ultraapi::graphql::{graphql_handler, graphiql};
//!
//! #[derive(SimpleObject)]
//! struct Query {
//!     hello: String,
//! }
//!
//! type MySchema = Schema<Query, async_graphql::EmptyMutation, async_graphql::EmptySubscription>;
//!
//! async fn graphql_endpoint(
//!     schema: Dep<MySchema>,
//!     req: axum::extract::Json<async_graphql::Request>,
//! ) -> impl IntoResponse {
//!     graphql_handler(&schema, req.0).await
//! }
//!
//! async fn graphiql_endpoint() -> impl IntoResponse {
//!     graphiql("/graphql", None).await
//! }
//! ```

#[cfg(feature = "graphql")]
mod graphql_impl {
    use async_graphql::Schema;
    use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
    use axum::response::{Html, IntoResponse};

    /// GraphQL endpoint handler
    ///
    /// This handler processes GraphQL queries via POST and returns JSON responses.
    ///
    /// # Type Parameters
    ///
    /// * `Q` - The query root type
    /// * `M` - The mutation root type
    /// * `S` - The subscription root type
    ///
    /// # Example
    ///
    /// ```ignore
    /// async fn graphql(
    ///     schema: Dep<Schema<Query, Mutation, EmptySubscription>>,
    ///     req: Json<Request>,
    /// ) -> impl IntoResponse {
    ///     graphql_handler(&schema, req.0).await
    /// }
    /// ```
    pub async fn graphql_handler<Q, M, S>(
        schema: &Schema<Q, M, S>,
        request: GraphQLRequest,
    ) -> GraphQLResponse
    where
        Q: async_graphql::ObjectType + Send + Sync + 'static,
        M: async_graphql::ObjectType + Send + Sync + 'static,
        S: async_graphql::SubscriptionType + Send + Sync + 'static,
    {
        schema.execute(request.into_inner()).await.into()
    }

    /// GraphQL POST endpoint that extracts the request from JSON body
    ///
    /// This is a convenience handler that combines extraction and execution.
    ///
    /// # Example
    ///
    /// ```ignore
    /// async fn graphql_post(
    ///     State(schema): State<Schema<Query, Mutation, EmptySubscription>>,
    ///     Json(req): Json<Request>,
    /// ) -> impl IntoResponse {
    ///     graphql_post_handler(&schema, req).await
    /// }
    /// ```
    pub async fn graphql_post_handler<Q, M, S>(
        schema: &Schema<Q, M, S>,
        request: async_graphql::Request,
    ) -> GraphQLResponse
    where
        Q: async_graphql::ObjectType + Send + Sync + 'static,
        M: async_graphql::ObjectType + Send + Sync + 'static,
        S: async_graphql::SubscriptionType + Send + Sync + 'static,
    {
        schema.execute(request).await.into()
    }

    /// GraphiQL endpoint handler
    ///
    /// Returns an HTML page with the GraphiQL interactive query editor.
    /// GraphiQL is a graphical interactive in-browser GraphQL IDE.
    ///
    /// # Example
    ///
    /// ```ignore
    /// async fn graphiql() -> impl IntoResponse {
    ///     graphiql("/graphql", None).await
    /// }
    /// ```
    pub async fn graphiql(
        graphql_endpoint: &str,
        _subscription_endpoint: Option<&str>,
    ) -> impl IntoResponse {
        let html = generate_graphiql_html(graphql_endpoint);
        Html(html)
    }

    /// Playground endpoint handler
    ///
    /// Returns an HTML page with the GraphQL Playground interactive query editor.
    /// GraphQL Playground is another graphical interactive in-browser GraphQL IDE.
    ///
    /// # Example
    ///
    /// ```ignore
    /// async fn playground() -> impl IntoResponse {
    ///     playground("/graphql", None).await
    /// }
    /// ```
    pub async fn playground(
        graphql_endpoint: &str,
        _subscription_endpoint: Option<&str>,
    ) -> impl IntoResponse {
        let html = generate_playground_html(graphql_endpoint);
        Html(html)
    }

    /// Generate GraphiQL HTML
    fn generate_graphiql_html(endpoint: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/graphiql/graphiql.min.css" />
  <script crossorigin src="https://cdn.jsdelivr.net/npm/graphiql/graphiql.min.js"></script>
</head>
<body style="margin: 0;">
  <div id="graphiql" style="height: 100vh;"></div>
  <script>
    const fetcher = GraphiQL.createFetcher({{
      url: '{}',
    }});
    ReactDOM.render(
      React.createElement(GraphiQL, {{ fetcher }}),
      document.getElementById('graphiql'),
    );
  </script>
</body>
</html>"#,
            endpoint
        )
    }

    /// Generate GraphQL Playground HTML
    fn generate_playground_html(endpoint: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <link rel="stylesheet" href="https://unpkg.com/graphplayground@1.5.0/style.css" />
  <script crossorigin src="https://unpkg.com/graphplayground@1.5.0/graphplayground.js"></script>
</head>
<body style="margin: 0;">
  <div id="playground" style="height: 100vh;"></div>
  <script>
    const playground = Graphplayground.createPlayground(document.getElementById('playground'), {{
      endpoint: '{}',
    }});
  </script>
</body>
</html>"#,
            endpoint
        )
    }
}

// Re-export graphql items at module level when feature is enabled
#[cfg(feature = "graphql")]
pub use graphql_impl::*;

#[cfg(not(feature = "graphql"))]
compile_error!("GraphQL feature is not enabled. Enable the `graphql` feature to use this module.");
