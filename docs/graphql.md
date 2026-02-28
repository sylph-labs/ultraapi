# GraphQL (optional)

UltraAPI includes **optional** GraphQL helpers powered by `async-graphql`.

- Feature flag: `graphql`
- HTTP integration: `async-graphql-axum`

> Note: UltraAPI does not attempt to model GraphQL operations in OpenAPI.

## Enable the feature

```toml
[dependencies]
ultraapi = { version = "0.1", features = ["graphql"] }
```

## Basic setup

You typically:

1. Build an `async_graphql::Schema`
2. Register it as an UltraAPI dependency via `.dep(schema)`
3. Add routes using `UltraApiApp::route_axum` (because GraphQL is not part of the standard UltraAPI route macro/OpenAPI modeling)

```rust
use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
use ultraapi::graphql::{graphiql, graphql_post_handler};
use ultraapi::prelude::*;

struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn hello(&self) -> &str {
        "Hello"
    }
}

type MySchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

async fn graphql_endpoint(
    schema: Dep<MySchema>,
    req: axum::extract::Json<async_graphql::Request>,
) -> impl axum::response::IntoResponse {
    graphql_post_handler(&*schema, req.0).await
}

async fn graphiql_endpoint() -> impl axum::response::IntoResponse {
    graphiql("/graphql", None).await
}

#[tokio::main]
async fn main() {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish();

    let app = UltraApiApp::new()
        .dep(schema)
        .route_axum("/graphql", axum::routing::post(graphql_endpoint))
        .route_axum("/graphiql", axum::routing::get(graphiql_endpoint));

    app.serve("0.0.0.0:3000").await;
}
```

## Example

A runnable example is included in the workspace:

- `examples/graphql-example`

Run it:

```bash
cd examples/graphql-example
cargo run
```

Then open:

- GraphiQL: <http://localhost:3000/graphiql>
- GraphQL endpoint: <http://localhost:3000/graphql>
