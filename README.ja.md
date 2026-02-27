# UltraAPI

[![crates.io](https://img.shields.io/crates/v/ultraapi.svg)](https://crates.io/crates/ultraapi)
[![docs.rs](https://docs.rs/ultraapi/badge.svg)](https://docs.rs/ultraapi)
[![CI](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml/badge.svg)](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml)

> English README: [README.en.md](./README.en.md)

Rust で **FastAPI ライクな開発体験（DX）** を目指す、Axum ベースの Web フレームワークです。

- コンセプト: **Rust 性能 × FastAPI DX**
- OpenAPI: `GET /openapi.json`
- Docs UI: `GET /docs`（既定は Embedded: Scalar）
- ReDoc UI: `GET /redoc`

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

## CLI（ultraapi コマンド）

UltraAPI には CLI ツール（`ultraapi` コマンド）が付属しています。

### インストール

```bash
cargo install ultraapi-cli
```

または、ultraapi ワークスペースから直接実行:

```bash
cargo run --bin ultraapi -- --help
```

### コマンド

#### アプリケーションの実行

```bash
# デフォルト設定で実行（0.0.0.0:3000）
ultraapi run ultraapi-example

# ホストとポートを指定
ultraapi run ultraapi-example --host 127.0.0.1 --port 8080

# 詳細な出力を有効にする
ultraapi -v run ultraapi-example --port 4000
```

#### 開発モード

```bash
# 開発モードで実行（現在のところ run と同じ動作）
ultraapi dev ultraapi-example --host 0.0.0.0 --port 3001
```

> **注意**: 自動リロード（auto-reload）機能は MVP では未実装です。

### 使用例

```bash
# examples/ultraapi-example をポート 3001 で起動
cargo run --bin ultraapi -- run ultraapi-example --port 3001

# 開発モードで起動
cargo run --bin ultraapi -- dev ultraapi-example --port 3001
```

## 主要マクロ

- ルート: `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`, `#[head]`, `#[options]`, `#[trace]`
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

Query/Form/Body の抽出時に自動で validate が実行され、バリデーション失敗時は 422（Unprocessable Entity）が返されます。

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

## レスポンス Cookie

UltraAPI は `CookieResponse<T>` を使用して、レスポンスに Set-Cookie ヘッダーを追加できます。FastAPI の `Response.set_cookie()` と同様の機能を提供します。

### 基本的な使用法

```rust
use ultraapi::prelude::*;

/// ライクセッションレスポンス
#[api_model]
#[derive(Debug, Clone)]
struct LoginResponse {
    status: String,
}

/// ログインページ
#[post("/login")]
#[response_class("cookie")]
async fn login() -> CookieResponse<LoginResponse> {
    CookieResponse::new(LoginResponse { status: "ok".to_string() })
        .cookie("session", "abc123")
}
```

### Cookie オプション

`cookie_options` メソッドを使用して、HttpOnly、Secure、SameSite、Path、Max-Age、Expires などのオプションを設定できます:

```rust
use ultraapi::prelude::*;
use time::OffsetDateTime;

/// 安全なセッション Cookie
#[post("/login/secure")]
#[response_class("cookie")]
async fn login_secure() -> CookieResponse<LoginResponse> {
    // 有効期限を7日後に設定
    let expires = OffsetDateTime::now_utc() + time::Duration::days(7);
    
    CookieResponse::new(LoginResponse { status: "ok".to_string() })
        .cookie_options("session", "abc123", |opts| {
            opts.http_only()      // JavaScript からアクセス不可
                .secure()          // HTTPS でのみ送信
                .path("/")         // サイト全体で有効
                .max_age(86400)    // 24時間有効
                .expires(expires)  // または絶対日時指定
        })
}
```

### 複数の Cookie

複数の Cookie を設定できます:

```rust
use ultraapi::prelude::*;

#[post("/login")]
#[response_class("cookie")]
async fn login() -> CookieResponse<LoginResponse> {
    CookieResponse::new(LoginResponse { status: "ok".to_string() })
        .cookie("session", "abc123")      // 基本的な Cookie
        .cookie("user_id", "42")         // 複数の Cookie
        .cookie_options("theme", "dark", |opts| {
            opts.same_site_lax()  // SameSite=Lax
        })
}
```

### 利用可能なオプション

- `http_only()` - HttpOnly フラグ（JavaScript からのアクセスをブロック）
- `secure()` - Secure フラグ（HTTPS でのみ送信）
- `same_site_strict()` - SameSite=Strict
- `same_site_lax()` - SameSite=Lax
- `same_site_none()` - SameSite=None（Secure が必要）
- `path(path)` - Cookie パス
- `max_age(seconds)` - 相対有効期限（秒）
- `expires(datetime)` - 絶対有効期限（time::OffsetDateTime）

## ファイルアップロード

UltraAPI は `Multipart` エクストラクタを使用してファイルアップロードをサポートしています。

### 単一ファイルアップロード

```rust
use ultraapi::prelude::*;
use axum::extract::Multipart;

/// アップロードレスポンス
#[api_model]
#[derive(Debug, Clone, Serialize)]
struct UploadResponse {
    filename: String,
    content_type: String,
    size: usize,
}

/// 単一ファイルアップロードエンドポイント
#[post("/upload")]
#[response_class("json")]
async fn upload_file(multipart: Multipart) -> Result<UploadResponse, ApiError> {
    // 最初のファイルフィールドを取得
    let mut multipart = multipart;
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(f)) if f.file_name().is_some() => break f,
            Ok(Some(_)) => continue, // ファイル以外のフィールドはスキップ
            Ok(None) => return Err(ApiError::bad_request("ファイルが見つかりません".to_string())),
            Err(e) => return Err(ApiError::bad_request(format!("Invalid multipart: {}", e))),
        }
    };

    let filename = field
        .file_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let content_type = field
        .content_type()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let data = field
        .bytes()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

    let size = data.len();

    Ok(UploadResponse {
        filename,
        content_type,
        size,
    })
}
```

### 複数ファイルアップロード

同じフィールド名で複数のファイルをアップロードできます:

```rust
use ultraapi::prelude::*;
use axum::extract::Multipart;

/// ファイル情報
#[api_model]
#[derive(Debug, Clone, Serialize)]
struct FileInfo {
    filename: String,
    content_type: String,
    size: usize,
}

/// 複数ファイルアップロードレスポンス
#[api_model]
#[derive(Debug, Clone, Serialize)]
struct MultipleUploadResponse {
    files: Vec<FileInfo>,
}

/// 複数ファイルアップロードエンドポイント
#[post("/upload/multiple")]
#[response_class("json")]
async fn upload_multiple_files(multipart: Multipart) -> Result<MultipleUploadResponse, ApiError> {
    let mut multipart = multipart;
    let mut files = Vec::new();

    // すべてのフィールド（ファイル）を処理
    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::bad_request(format!("Invalid multipart: {}", e)))? {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

        let size = data.len();

        files.push(FileInfo {
            filename,
            content_type,
            size,
        });
    }

    Ok(MultipleUploadResponse { files })
}
```

### ファイルとメタデータの同時アップロード

テキストフィールドとファイルを同時に送信できます:

```rust
use ultraapi::prelude::*;
use axum::extract::Multipart;

#[post("/upload/with-meta")]
#[response_class("json")]
async fn upload_file_with_metadata(
    multipart: Multipart,
) -> Result<UploadResponse, ApiError> {
    let mut multipart = multipart;
    
    let mut filename = "default.txt".to_string();
    let mut content_type = "text/plain".to_string();
    let mut size = 0usize;

    while let Some(field) = multipart.next_field().await.map_err(|e| ApiError::bad_request(format!("Invalid multipart: {}", e)))? {
        let field_name = field.name().unwrap_or_default();

        if field_name == "description" {
            // 説明フィールドはスキップ
            let _ = field.text().await;
        } else if field_name == "file" {
            // ファイルフィールドを処理
            filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            content_type = field
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file: {}", e)))?;

            size = data.len();
        }
    }

    Ok(UploadResponse {
        filename,
        content_type,
        size,
    })
}
```

## グローバルエラーハンドリング

UltraAPI では、カスタム例外をグローバルに処理するためのエラーハンドラーを登録できます。

### カスタム例外の定義

```rust
use ultraapi::prelude::*;
use axum::http::StatusCode;

/// ビジネスロジック用のカスタム例外
#[derive(Debug, Clone)]
struct BusinessException {
    code: String,
    message: String,
}

impl BusinessException {
    fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}
```

### グローバルエラーハンドラーの登録

```rust
use std::sync::Arc;
use axum::{body::Body, http::Request, response::IntoResponse, http::StatusCode};

fn make_error_handler() -> CustomErrorHandler {
    Arc::new(|_state: AppState, _req: Request<Body>, error: Box<dyn std::any::Any + Send + 'static>| {
        Box::pin(async move {
            // カスタム例外类型をダウンキャストして処理
            if let Some(ex) = error.downcast_ref::<BusinessException>() {
                let body = serde_json::json!({
                    "error": "BusinessError",
                    "code": ex.code,
                    "message": ex.message
                });
                return (StatusCode::BAD_REQUEST, serde_json::to_string(&body).unwrap()).into_response();
            }
            // デフォルトのエラー応答
            (StatusCode::INTERNAL_SERVER_ERROR, r#"{"error":"Unknown error"}"#).into_response()
        })
    })
}

// アプリを作成する際にエラーハンドラーを登録
let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .error_handler_from_arc(make_error_handler());
```

### パニックキャッチの有効化

パニックが発生した場合にサーバー全体がクラッシュするのを防ぐため、`catch_panic()` メソッドを使用できます:

```rust
let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .catch_panic();  // パニックをキャッチして500エラーを返す
```

### 複数のオプションのチェーン

エラーハンドラーとパニックキャッチを組み合わせることも可能:

```rust
let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .error_handler_from_arc(make_error_handler())
    .catch_panic();
```

## レスポンス圧縮（Compression / GZip / Brotli）

UltraAPI では、レスポンスを自動的に圧縮するミドルウェアを有効にできます。クライアントが `Accept-Encoding: gzip` や `Accept-Encoding: br` を送信した場合、サーバーは圧縮されたレスポンスを返します。

### 基本的な使い方

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .gzip();  // gzip + brotli 圧縮を有効化
```

### カスタム設定

圧縮アルゴリズムを個別に制御できます:

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::CompressionConfig;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .compression(
        CompressionConfig::new()
            .gzip(true)      // gzip を有効
            .brotli(false)   // brotli を無効
            .deflate(false)  // deflate を無効
    );
```

### 動作

- クライアントが `Accept-Encoding` ヘッダーを送信しない場合、圧縮は行われません
- 小さいレスポンス（デフォルト閾値以下）は圧縮されない場合があります
- `Accept-Encoding: identity` の場合は圧縮されません

## テストクライアント（TestClient）

UltraAPI には、FastAPI ライクな `TestClient` が組み込まれています。サーバーを手動で起動せずに HTTP リクエストをテストできます。

### 基本的な使用法

```rust
use ultraapi::prelude::*;

// モデル定義
#[api_model]
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
}

// ルート定義
#[get("/users/{id}")]
async fn get_user(id: i64) -> User {
    User { id, name: "Alice".to_string() }
}

// テスト
#[tokio::test]
async fn test_get_user() {
    let app = UltraApiApp::new();
    let client = TestClient::new(app).await;
    
    let response = client.get("/users/42").await;
    assert_eq!(response.status(), 200);
    
    let user: User = response.json().await.unwrap();
    assert_eq!(user.id, 42);
}
```

### 対応HTTPメソッド

- `get(path)` - GET リクエスト
- `post(path, &body)` - POST リクエスト（JSON）
- `put(path, &body)` - PUT リクエスト（JSON）
- `delete(path)` - DELETE リクエスト
- `patch(path, &body)` - PATCH リクエスト（JSON）
- `head(path)` - HEAD リクエスト
- `client()` - 基盤の `reqwest::Client` を取得（カスタムリクエスト用）

### UltraApiApp または Router から作成

```rust
// UltraApiApp から
let app = UltraApiApp::new().title("My API");
let client = TestClient::new(app).await;

// Router から
let router = UltraApiApp::new().into_router();
let client = TestClient::new_router(router).await;
```

## Examples

- `examples/ultraapi-example`
- `examples/grpc-example`

## ライセンス

MIT
