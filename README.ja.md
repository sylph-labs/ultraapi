# UltraAPI

[![crates.io](https://img.shields.io/crates/v/ultraapi.svg)](https://crates.io/crates/ultraapi)
[![docs.rs](https://docs.rs/ultraapi/badge.svg)](https://docs.rs/ultraapi)
[![CI](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml/badge.svg)](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml)

> English README: [README.en.md](./README.en.md)

Rust で **FastAPI ライクな開発体験（DX）** を目指す、Axum ベースの Web フレームワークです。

- コンセプト: **Rust 性能 × FastAPI DX**
- OpenAPI: `GET /openapi.json`
- Docs UI: `GET /docs`（既定は Embedded: Scalar）

## 特徴（MVP）

- **FastAPI 風のルート定義**: `#[get]`, `#[post]`, `#[put]`, `#[delete]`
- **serde/schemars から OpenAPI 自動生成**: `#[api_model]` の型定義から schema を生成
- **/docs 組み込み**: そのまま API リファレンス UI を提供（CDN Swagger UI も可）
- **自動バリデーション**: `#[validate(...)]` で 422（Unprocessable Entity）を返却
- **DI（依存性注入）**: `Dep<T>`, `State<T>`, `Depends<T>`
- **Router 合成**: prefix / tags / security をルーター単位で合成
- **WebSocket / SSE**: `#[ws]`, `#[sse]`
- **Lifespan hooks**: startup/shutdown

## インストール

```toml
[dependencies]
ultraapi = "0.1"
```

## クイックスタート

### 1) モデル定義（OpenAPI + Validation）

```rust
use ultraapi::prelude::*;

/// ユーザー作成リクエスト
#[api_model]
#[derive(Debug, Clone)]
struct CreateUser {
    #[validate(min_length = 1, max_length = 100)]
    name: String,

    #[validate(email)]
    email: String,
}

/// ユーザー
#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
}
```

### 2) ルート定義（FastAPI 風）

```rust
use ultraapi::prelude::*;

#[post("/users")]
async fn create_user(body: CreateUser) -> User {
    User { id: 1, name: body.name, email: body.email }
}

#[get("/users/{id}")]
async fn get_user(id: i64) -> Result<User, ApiError> {
    Ok(User { id, name: "Alice".into(), email: "alice@example.com".into() })
}
```

### 3) Router 合成 + 起動

`#[get]` などのマクロは、ルート参照（`__HAYAI_ROUTE_<FN>`）を自動生成します。

```rust
use ultraapi::prelude::*;

fn api() -> UltraApiRouter {
    UltraApiRouter::new("/api")
        .tag("users")
        .route(__HAYAI_ROUTE_CREATE_USER)
        .route(__HAYAI_ROUTE_GET_USER)
}

#[tokio::main]
async fn main() {
    UltraApiApp::new()
        .title("My API")
        .version("1.0.0")
        .include(api())
        .serve("0.0.0.0:3000")
        .await;
}
```

起動後:

- OpenAPI: `GET /openapi.json`
- Docs: `GET /docs`

## 主要マクロ

- ルート: `#[get]`, `#[post]`, `#[put]`, `#[delete]`
- モデル: `#[api_model]`
- WebSocket: `#[ws]`
- SSE: `#[sse]`

### ルート向け追加属性

- `#[status(200)]` など: 成功時ステータスコード
- `#[tag("name")]`: OpenAPI タグ
- `#[security("bearer")]`: セキュリティ要件（OpenAPI と auth middleware に反映）
- `#[response_class("json"|"html"|"text"|"binary"|"stream"|"xml")]`: content-type
- `#[response_model(...)]`: response shaping（include/exclude/by_alias）
- `#[summary("...")]`: OpenAPI summary
- `#[external_docs(url = "...", description = "...")]`: OpenAPI externalDocs
- `#[deprecated]`: OpenAPI deprecated

### モデルフィールド向け属性

`#[api_model]` を付けたstructのフィールドには以下の属性が使えます:

- `#[read_only]`: レスポンス에만 포함되고、リクエストには 포함되지まないフィールド（OpenAPI で `readOnly: true` を出力）
- `#[write_only]`: リクエストだけに 포함되고、レスポンスには 포함되지まないフィールド（OpenAPI で `writeOnly: true` を出力）
- `#[alias("name")]`: フィールドのシリアライズ名を指定（serde の `rename` 相当）

#### read_only / write_only の使用例

```rust
use ultraapi::prelude::*;

/// ユーザー作成リクエスト（パスワードのみリクエスト時に必要）
#[api_model]
#[derive(Debug, Clone)]
struct CreateUser {
    /// ユーザー名
    name: String,

    /// パスワード（リクエスト時のみ、レスポンスでは返さない）
    #[write_only]
    password: String,
}

/// ユーザーレスポンス（IDはレスポンスでのみ返す）
#[api_model]
#[derive(Debug, Clone)]
struct User {
    /// ユーザーID（レスポンスのみ）
    #[read_only]
    id: i64,

    /// ユーザー名
    name: String,
}
```

- `#[read_only]` を付けたフィールドは、リクエストボディのデシリアライズ時に無視されます
- `#[write_only]` を付けたフィールドは、レスポンスのシリアライズ時に除外されます
- OpenAPI の Schema プロパティにはそれぞれ `readOnly: true` / `writeOnly: true` が出力されます

## Swagger UI / Docs

既定は Embedded（Scalar）です。Swagger UI を CDN から読み込みたい場合:

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new().swagger_cdn("https://unpkg.com/swagger-ui-dist@5");
```

## Webhooks と Callbacks（OpenAPI）

UltraAPI は OpenAPI 3.1 の **webhooks** と **callbacks** をサポートしています。

- `webhooks` は OpenAPI spec のトップレベル `webhooks` に出力
- `callbacks` は特定の operation の `callbacks` に出力

これらの API は **OpenAPI への出力** を追加します（runtime router への登録は行いません）。
ただし、最終的にルートが公開されるかどうかは **アプリのルーティング方式** に依存します。

- **explicit routing（`.include(...)` を使う場合）**: include していないルートは runtime に登録されません
- **implicit routing（inventory 全登録を使う場合）**: `#[get]`/`#[post]` などで定義したルートは runtime に登録されます

「OpenAPI にだけ載せたい」場合は、explicit routing を使い、webhook/callback 用ルートを `include(...)` しない運用にしてください。

### Webhooks

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone)]
struct PaymentEvent {
    event_type: String,
    amount: f64,
}

#[post("/webhooks/payment")]
#[tag("webhooks")]
async fn payment_webhook(body: PaymentEvent) -> PaymentEvent {
    body
}

let app = UltraApiApp::new()
    .webhook("payment", __HAYAI_ROUTE_PAYMENT_WEBHOOK);
```

### Callbacks

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone)]
struct Subscription {
    id: i64,
    plan: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct SubscriptionEvent {
    event_type: String,
    subscription_id: i64,
}

#[post("/subscriptions")]
async fn create_subscription(body: Subscription) -> Subscription {
    body
}

#[post("/webhooks/subscription")]
async fn subscription_callback(body: SubscriptionEvent) -> SubscriptionEvent {
    body
}

let app = UltraApiApp::new().callback(
    __HAYAI_ROUTE_CREATE_SUBSCRIPTION,
    "subscriptionEvent",
    "{$request.body#/callbackUrl}",
    __HAYAI_ROUTE_SUBSCRIPTION_CALLBACK,
);
```

## バリデーション

`#[api_model]` を付けた型に対して、以下の属性が利用できます:

- `#[validate(email)]`
- `#[validate(min_length = N)]`
- `#[validate(max_length = N)]`
- `#[validate(minimum = N)]`
- `#[validate(maximum = N)]`
- `#[validate(pattern = "...")]`
- `#[validate(min_items = N)]`

## 依存性注入（DI）

- `Dep<T>` / `State<T>`: アプリに登録した依存性を取り出す
- `Depends<T>`: FastAPI 風の依存性（関数ベース）
- `yield_depends`: cleanup を持つ依存性（scope: Function/Request）

## Sub Applications（mount）

UltraAPI は FastAPI ライクなサブアプリケーション（サブアプリ）をサポートしています。

```rust
use ultraapi::prelude::*;

// サブアプリを作成
let sub_app = UltraApiApp::new()
    .title("Sub API")
    .version("1.0.0");

// メインアプリにマウント
let app = UltraApiApp::new()
    .mount("/api", sub_app);
```

サブアプリは以下の特徴を持ちます:
- 独自の `/docs` と `/openapi.json` エンドポイント（`/api/docs`, `/api/openapi.json`）
- メインアプリの OpenAPI にはサブアプリのルートは含まれません（分離）
- メインアプリと依存性を共有します

## Static Files

静的ファイル（画像、CSS、JS など）を配信できます:

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new()
    .static_files("/static", "./static");
```

- 第1引数: URL パスプレフィックス（例: `/static`）
- 第2引数: 配信するディレクトリのパス

## Templates

HTML テンプレート（Jinja2 形式）をレンダリングできます:

```rust
use ultraapi::prelude::*;
use ultraapi::templates::{Templates, template_response};

// テンプレートディレクトリを設定
let app = UltraApiApp::new()
    .templates_dir("./templates");

// ハンドラーでテンプレートを使用
#[get("/hello")]
async fn hello(templates: Dep<Templates>) -> impl IntoResponse {
    template_response(&templates, "hello.html", serde_json::json!({ "name": "World" }))
}
```

テンプレート機能:
- `Templates::new(dir)` - テンプレートディレクトリからTemplatesを作成
- `Templates::render(name, context)` - テンプレートをレンダリング
- `template_response(templates, name, context)` - HTMLレスポンスを生成
- `TemplateResponse` 型は `IntoResponse` を実装し、自動的に `text/html` content-type を設定

## Examples

- `examples/ultraapi-example`
- `examples/grpc-example`

## ライセンス

MIT
