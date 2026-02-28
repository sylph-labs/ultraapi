//! GraphQL Example for UltraAPI
//!
//! This example demonstrates how to create a GraphQL endpoint with UltraAPI.
//! It provides a simple Query { hello: String } endpoint.
//!
//! Run with:
//! ```sh
//! cd examples/graphql-example
//! cargo run
//! ```
//!
//! Then visit:
//! - GraphiQL: http://localhost:3000/graphiql
//! - GraphQL endpoint: http://localhost:3000/graphql

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
use ultraapi::prelude::*;

// Define the Query root type
#[derive(Default)]
struct Query;

#[Object]
impl Query {
    /// A simple greeting
    async fn hello(&self) -> &str {
        "Hello, world!"
    }

    /// A greeting with a name
    async fn hello_name(&self, #[graphql(desc = "The name to greet")] name: String) -> String {
        format!("Hello, {}!", name)
    }
}

// Create the schema type
type MySchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// GraphQL endpoint handler using axum's State extractor with AppState
async fn graphql_handler(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    req: axum::extract::Json<async_graphql::Request>,
) -> impl axum::response::IntoResponse {
    // Extract the schema from AppState
    let schema = state
        .get::<MySchema>()
        .expect("GraphQL schema not registered");
    let response = schema.execute(req.0).await;
    axum::response::Json(response)
}

/// GraphiQL endpoint handler
async fn graphiql_handler() -> impl axum::response::IntoResponse {
    use axum::response::Html;
    Html(generate_graphiql_html("/graphql"))
}

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

// Re-export AppState for the handler
type AppState = ultraapi::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the GraphQL schema
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();

    println!("Starting GraphQL server...");
    println!("GraphiQL available at: http://localhost:3000/graphiql");
    println!("GraphQL endpoint at: http://localhost:3000/graphql");

    // Create the UltraAPI app with the schema as a dependency
    // and register the GraphQL routes using route_axum
    let app = UltraApiApp::new()
        .title("GraphQL Example")
        .version("0.1.0")
        .dep(schema)
        .route_axum("/graphql", axum::routing::post(graphql_handler))
        .route_axum("/graphiql", axum::routing::get(graphiql_handler));

    // Run the server
    app.serve("0.0.0.0:3000").await;

    Ok(())
}
