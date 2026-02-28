# UltraAPI

[![crates.io](https://img.shields.io/crates/v/ultraapi.svg)](https://crates.io/crates/ultraapi)
[![docs.rs](https://docs.rs/ultraapi/badge.svg)](https://docs.rs/ultraapi)
[![CI](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml/badge.svg)](https://github.com/sylph-labs/ultraapi/actions/workflows/ci.yml)

> English README: [README.md](./README.md)

Rust で **FastAPI ライクな開発体験（DX）** を目指す、Axum ベースの Web フレームワークです。

- コンセプト: **Rust 性能 × FastAPI DX**
- OpenAPI: `GET /openapi.json`
- Docs UI: `GET /docs`（既定は Embedded: Scalar）
- ReDoc UI: `GET /redoc`

## 特徴

- **FastAPI 風のルート定義**: `#[get]`, `#[post]`, `#[put]`, `#[delete]`
- **serde/schemars から OpenAPI 自動生成**: `#[api_model]` の型定義から schema を生成
- **/docs 組み込み**: そのまま API リファレンス UI を提供（CDN Swagger UI も可）
- **自動バリデーション**: `#[validate(...)]` で 422（Unprocessable Entity）を返却
- **DI（依存性注入）**: `Dep<T>`, `State<T>`, `Depends<T>`
- **Router 合成**: prefix / tags / security をルーター単位で合成
- **WebSocket / SSE**: `#[ws]`, `#[sse]`
- **Lifespan hooks**: startup/shutdown

## Lifespan（起動/終了フック）

UltraAPI は、アプリケーションの起動時と終了時に実行されるフックをサポートしています。

### 基本的な使用例

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new()
    .lifecycle(|lifecycle| {
        lifecycle
            .on_startup(|state| {
                Box::pin(async move {
                    println!("アプリケーション起動中...");
                    // データベース接続の確立
                    // テンプレートのロード
                })
            })
            .on_shutdown(|state| {
                Box::pin(async move {
                    println!("アプリケーション終了中...");
                    // データベース接続のクローズ
                    // リソースのクリーンアップ
                })
            })
    });
```

### 3つの利用パターン

#### 1. `serve()` を使用する場合（推奨）

`serve()` メソッドを使用する場合、startup フックはサーバー起動時に実行され、shutdown フックは Ctrl+C によるgraceful shutdown 時に実行されます。

```rust
#[tokio::main]
async fn main() {
    UltraApiApp::new()
        .lifecycle(|l| l
            .on_startup(|_| Box::pin(async { println!(" startup!"); }))
            .on_shutdown(|_| Box::pin(async { println!(" shutdown!"); }))
        )
        .serve("0.0.0.0:3000")
        .await;
}
```

#### 2. `TestClient` を使用する場合（テスト用）

テストでは `TestClient` が自動的に lifecycle を管理します。startup は最初のリクエスト時に実行され、shutdown はテスト終了時（Drop 時）に実行されます。

```rust
#[tokio::test]
async fn test_my_api() {
    let app = UltraApiApp::new()
        .lifecycle(|l| l
            .on_startup(|_| Box::pin(async { /* テスト用リソース */ }))
            .on_shutdown(|_| Box::pin(async { /* クリーンアップ */ }))
        );
    
    let client = TestClient::new(app).await;
    
    // リクエストを実行（この時点で startup が実行される）
    let response = client.get("/api/items").await;
    
    // テスト終了時に shutdown が自動的に呼ばれる
    // または、明示的に呼ぶことも可能
    client.shutdown().await;
}
```

#### 3. `into_router_with_lifespan()` を使用する場合

router を直接使用する場合（カスタムサーバーや他の用途）は、`into_router_with_lifespan()` を使用して lifecycle を統合できます。

```rust
let app = UltraApiApp::new()
    .lifecycle(|l| l
        .on_startup(|_| Box::pin(async { /* 起動処理 */ }))
        .on_shutdown(|_| Box::pin(async { /* 終了処理 */ }))
    );

let (router, runner) = app.into_router_with_lifespan();

// router を使用してサーバーを起動...
// 例: axum::serve(listener, router).await

// 終了時に shutdown を手動でトリガー
runner.shutdown().await;
```

### 注意事項

- **複数回実行の防止**: startup フックは最初のリクエスト時に一度だけ実行されます。重複実行を防ぐため、内部で適切なロックを使用しています。
- **`into_router()` との併用**: 通常の `into_router()` メソッドは lifecycle を統合しません。lifecycle を使用する場合は `into_router_with_lifespan()` を使用してください。
- **lazy startup**: `into_router_with_lifespan()` と `TestClient` の場合、startup は最初のリクエスト時に実行されます（lazy startup）。

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
- `#[security("basicAuth")]`: Basic認証（OpenAPI と auth middleware に反映）
- `#[security("oauth2Password")]`: OAuth2 Password Flow（OpenAPI に反映）
- `#[security("oauth2AuthCode")]`: OAuth2 Authorization Code Flow（OpenAPI に反映）
- `#[security("oauth2Implicit")]`: OAuth2 Implicit Flow（OpenAPI に反映）

#### OAuth2 依存オブジェクト

UltraAPIはFastAPI互換のOAuth2依存オブジェクトを提供します：

```rust
use ultraapi::prelude::*;

/// OAuth2PasswordBearer: auto_error=true（デフォルト）
/// トークンが 없는場合は401エラーを返す
#[get("/protected")]
async fn protected_endpoint(token: OAuth2PasswordBearer) -> String {
    format!("Token: {}", token.0)
}

/// OptionalOAuth2PasswordBearer: auto_error=false
/// トークンがなくてもエラーではなくNoneを返す
#[get("/optional-protected")]
async fn optional_protected_endpoint(token: OptionalOAuth2PasswordBearer) -> String {
    match token.0 {
        Some(t) => format!("Token: {}", t),
        None => "No token provided".to_string(),
    }
}

/// OAuth2AuthorizationCodeBearer: Authorization Code Flow用
#[get("/auth-code-protected")]
async fn auth_code_protected_endpoint(token: OAuth2AuthorizationCodeBearer) -> String {
    format!("Auth Code Token: {}", token.0)
}
```

これらの依存オブジェクトを使用するには、セキュリティスキームをアプリに登録する必要があります：

```rust
let app = UltraApiApp::new()
    .title("OAuth2 API")
    .version("0.1.0")
    .oauth2_password(
        "oauth2Password",
        "https://example.com/token",
        [("read", "Read access"), ("write", "Write access")],
    )
    // または
    .oauth2_authorization_code(
        "oauth2AuthCode",
        "https://example.com/authorize",
        "https://example.com/token",
        [("read", "Read access")],
    );
```

- `OAuth2PasswordBearer` / `OptionalOAuth2PasswordBearer`: OAuth
- `OAuth2AuthorizationCodeBearer` / `OptionalOAuth2AuthorizationCode2 Password Flow用Bearer`: OAuth2 Authorization Code Flow用
- `auto_error=true`（デフォルト）: トークンがない場合は401を返す
- `auto_error=false`（Optional* バージョン）: トークンがなくても200でNoneを返す

#### OAuth2 実運用コンポーネント

UltraAPIはOAuth2の実運用で必要となる型とヘルパを提供します。これらは `ultraapi::oauth2` または `ultraapi::prelude` からアクセスできます。

```rust
use ultraapi::oauth2::{
    OAuth2PasswordRequestForm,
    TokenResponse,
    OAuth2ErrorResponse,
    OpaqueTokenValidator,
};
```

##### /token エンドポイントの実装例

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::create_bearer_auth_error;

#[post("/token")]
async fn token(
    Form(form): Form<OAuth2PasswordRequestForm>,
) -> Result<Json<TokenResponse>, Json<OAuth2ErrorResponse>> {
    // パスワードグラントのみサポート
    if !form.is_password_grant() {
        return Err(Json(OAuth2ErrorResponse::unsupported_grant_type()));
    }
    
    // ユーザー認証（実際にはデータベースなどで検証）
    let valid = verify_credentials(&form.username, &form.password);
    if !valid {
        return Err(Json(OAuth2ErrorResponse::invalid_grant(
            "Invalid username or password"
        )));
    }
    
    // トークン生成
    let access_token = generate_token(&form.username, form.scopes());
    let response = TokenResponse::with_scopes(access_token, 3600, form.scopes());
    
    Ok(Json(response))
}
```

##### カスタムバリデーターの実装

独自のトークンバリデーターを実装するには `OAuth2TokenValidator` trait を使用します：

```rust
use ultraapi::middleware::{OAuth2TokenValidator, TokenData, OAuth2AuthError};

struct MyTokenValidator;

#[async_trait::async_trait]
impl OAuth2TokenValidator for MyTokenValidator {
    async fn validate(&self, token: &str) -> Result<TokenData, OAuth2AuthError> {
        // 独自の検証ロジックを実装
        // （JWTデコード、データベース参照、Redis参照など）
        
        Ok(TokenData::new("user123".to_string(), vec!["read".to_string()]))
    }
}
```

##### 不透明トークンバリデーター（Opaque Token Validator）

テストや単純な用途向けに `OpaqueTokenValidator` が含まれています：

```rust
use ultraapi::oauth2::OpaqueTokenValidator;

// トークンを追加
let validator = OpaqueTokenValidator::new()
    .add_token("valid-token-1", "user1", vec!["read".to_string()])
    .add_token("valid-token-2", "user2", vec!["read".to_string(), "write".to_string()]);

// トークン検証
let result = validator.validate("valid-token-1").await;
match result {
    Ok(token_data) => {
        println!("User: {}", token_data.sub);
        println!("Scopes: {:?}", token_data.scopes());
    }
    Err(e) => {
        println!("Invalid token: {}", e);
    }
}

// スコープ検証
let token_data = validator.validate("valid-token-2").await.unwrap();
let result = validator.validate_scopes(&token_data, &["read".to_string()]);
// result Ok if user has "read" scope
```

##### 含まれる型

| 型 | 説明 |
|---|---|
| `OAuth2PasswordRequestForm` | パスワードフローのリクエストフォーム |
| `TokenResponse` | 成功時のトークンレスポンス |
| `OAuth2ErrorResponse` | RFC 6749 準拠のエラーレスポンス |
| `TokenData` | 検証されたトークンデータ |
| `OAuth2AuthError` | トークン検証エラー |
| `OAuth2TokenValidator` | バリデーター trait |
| `OpaqueTokenValidator` | 不透明トークンバリデーターの実装例 |
| `create_bearer_auth_error` | Bearer 認証エラー応答ヘルパー |

##### security 属性と middleware の関係

`#[security("oauth2Password")]` を使用する場合:

1. OpenAPI の securityScheme に oauth2Password が追加される
2. ミドルウェアが Authorization ヘッダを確認し、Bearer トークンを抽出
3. トークンは `OAuth2PasswordBearer` 依存オブジェクトとしてルートに渡される
4. カスタムバリデーターを使用する場合、`AuthLayer` または `AuthValidator` を設定

スコープが必要な場合:
```rust
#[get("/admin")]
#[security("oauth2Password:admin")]
async fn admin_endpoint(token: OAuth2PasswordBearer) -> String {
    // "admin" スコープが必要
    format!("Admin access for: {}", token.0)
}
```

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

## ストリームレスポンス (StreamingResponse)

UltraAPI は `StreamingResponse` を提供し、FastAPI の `StreamingResponse` と同等の機能を実現します。任意のストリームを HTTP レスポンスとして返す際に使用します。

### 特徴

- 任意の `impl Stream<Item = Result<Bytes, E>>` または `impl Stream<Item = Bytes>` を受け取る
- Content-Type（media_type）の指定が可能
- カスタムヘッダの追加が可能
- ステータスコードの指定が可能
- エラーハンドリング: ストリーム内のエラーはログに出力され、接続は閉じられます

### 基本的な使用法

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;

/// ストリームエンドポイント
#[get("/stream")]
async fn stream_handler() -> StreamingResponse {
    let stream = iter([
        Ok::<_, std::convert::Infallible>(Bytes::from("chunk1\n")),
        Ok(Bytes::from("chunk2\n")),
        Ok(Bytes::from("chunk3\n")),
    ]);
    StreamingResponse::from_infallible_stream(stream)
}
```

### カスタム Content-Type

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;

/// テキストストリーム
#[get("/stream/text")]
async fn text_stream() -> StreamingResponse {
    let stream = iter([
        Ok(Bytes::from("line1\n")),
        Ok(Bytes::from("line2\n")),
        Ok(Bytes::from("line3\n")),
    ]);
    StreamingResponse::from_infallible_stream(stream)
        .content_type("text/plain")
}
```

### カスタムヘッダー

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;

/// カスタムヘッダー付きストリーム
#[get("/stream/headers")]
async fn stream_with_headers() -> StreamingResponse {
    let stream = iter([Ok(Bytes::from("data"))]);
    StreamingResponse::from_infallible_stream(stream)
        .header("X-Custom-Header", "custom-value")
        .header("X-Request-Id", "12345")
}
```

### カスタムステータスコード

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;
use axum::http::StatusCode;

/// パート内容レスポンス
#[get("/stream/partial")]
async fn partial_stream() -> StreamingResponse {
    let stream = iter([Ok(Bytes::from("partial content"))]);
    StreamingResponse::from_infallible_stream(stream)
        .status(StatusCode::PARTIAL_CONTENT)
}
```

### すべてのオプションを組み合わせ

```rust
use ultraapi::prelude::*;
use tokio_stream::iter;
use axum::http::StatusCode;

/// フルオプション付きストリーム
#[get("/stream/full")]
async fn full_stream() -> StreamingResponse {
    let stream = iter([Ok(Bytes::from("full response"))]);
    StreamingResponse::from_infallible_stream(stream)
        .content_type("application/json")
        .header("X-Request-Id", "12345")
        .status(StatusCode::OK)
}
```

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

### FastAPI 互換の GZip 設定（推奨）

FastAPI の `GZipMiddleware` に近い設定として、`minimum_size`（圧縮する最小サイズ）や、圧縮対象の `content_types` を指定できます。

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::GZipConfig;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .gzip_config(
        GZipConfig::new()
            .minimum_size(1024)
            .content_types(vec![
                "text/*".to_string(),
                "application/json".to_string(),
            ]),
    );
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

## セッション（Session Cookies / サーバーサイドセッション）

UltraAPI は Cookie + サーバー側 in-memory store による **サーバーサイドセッション** を提供します。

- Cookie には `session_id` のみを保存します
- セッションデータはサーバー側に保存されます
- TTL による期限切れをサポートします

```rust
use ultraapi::prelude::*;
use std::time::Duration;

#[get("/login")]
async fn login(session: Session) -> String {
    session.insert("user_id", 123_i64).unwrap();
    "ok".to_string()
}

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .session_cookies(SessionConfig::new("dev-secret").ttl(Duration::from_secs(3600)));
```

## JWT（AuthLayer validator）ガイド

JWT を `AuthLayer` の validator として統合する手順は `docs/jwt.ja.md` を参照してください。

- `docs/jwt.ja.md`

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
// UltraApiApp から（TCP で起動する通常版）
let app = UltraApiApp::new().title("My API");
let client = TestClient::new(app).await;

// Router から（TCP で起動する通常版）
let router = UltraApiApp::new().into_router();
let client = TestClient::new_router(router).await;
```

### in-process（推奨：高速）

ネットワークの bind をせず、`axum::Router`（tower::Service）を **直接呼び出す** テストクライアントも提供しています。

```rust
use ultraapi::prelude::*;

#[get("/hello")]
async fn hello() -> String {
    "Hello".to_string()
}

#[tokio::test]
async fn test_hello_in_process() {
    let app = UltraApiApp::new().include(
        UltraApiRouter::new("").route(__ULTRAAPI_ROUTE_HELLO)
    );

    let client = TestClient::new_in_process(app).await;

    let resp = client.get("/hello").await;
    assert_eq!(resp.status(), 200);
}
```

- 戻り値は in-process 用の `TestResponse`（`status()` / `headers()` / `bytes()` / `text()` / `json()`）です
- in-process 版は TCP 起動がないため、テストが大幅に速くなります

## 実装済み機能一覧

### コア機能
- ✅ FastAPI 風ルートマクロ（`#[get]`、`#[post]`、`#[put]`、`#[delete]`、`#[patch]`、`#[head]`、`#[options]`、`#[trace]`）
- ✅ 自動 OpenAPI 3.1 生成
- ✅ 組み込み Swagger UI（`/docs`）および ReDoc（`/redoc`）
- ✅ スキーマ生成のための serde/schemars 統合
- ✅ `#[validate]` 属性による自動バリデーション
- ✅ 依存性注入（`Dep<T>`、`State<T>`、`Depends<T>`）
- ✅ クリーンアップ付き yield 依存性（Function/Request スコープ）
- ✅ プレフィックス/タグ/セキュリティ伝播によるルーター合成

### 認証・セキュリティ
- ✅ Bearer 認証（JWT）
- ✅ API キー認証（ヘッダー/クエリ/クッキー）
- ✅ OAuth2 フロー（Implicit、Password、Client Credentials、Authorization Code）
- ✅ OpenID Connect
- ✅ ミドルウェアによるランタイム認証強制
- ✅ スコープベースの認証
- ✅ OAuth2 依存オブジェクト（`OAuth2PasswordBearer`、`OptionalOAuth2PasswordBearer` など）
- ✅ OAuth2 実運用コンポーネント（`OAuth2PasswordRequestForm`、`TokenResponse` など）

### レスポンス処理
- ✅ レスポンスモデルシェイピング（include/exclude/by_alias）
- ✅ レスポンスクラス指定（json/html/text/binary/stream/xml）
- ✅ フィールドレベル属性（`#[read_only]`、`#[write_only]`、`#[alias]`）
- ✅ `#[status]` によるカスタムステータスコード
- ✅ カスタム例外によるグローバルエラーハンドリング
- ✅ パニックキャッチ
- ✅ レスポンス圧縮（GZip/Brotli）
- ✅ レスポンスキャッシュ（TTL / x-cache / Authorization はバイパス）
- ✅ ストリーミングデータ用 StreamingResponse
- ✅ Cookie 設定用 CookieResponse

### 高度な機能
- ✅ 3 つの利用パターンを持つ Lifespan フック（startup/shutdown）
- ✅ WebSocket サポート（`#[ws]`）
- ✅ SSE サポート（`#[sse]`）
- ✅ Webhooks（OpenAPI 3.1）
- ✅ Callbacks（OpenAPI 3.1）
- ✅ サブアプリケーション（mount）
- ✅ 静的ファイル配信
- ✅ Jinja2 風テンプレート
- ✅ ファイルアップロード（Multipart）
- ✅ テスト用 TestClient

### 開発者ツール
- ✅ アプリケーション実行用 CLI（`ultraapi` コマンド）
- ✅ 開発モード
- ✅ FastAPI との OpenAPI パリティのためのゴールデンテスト

## Examples

- `examples/ultraapi-example`
- `examples/grpc-example`

## ライセンス

MIT
