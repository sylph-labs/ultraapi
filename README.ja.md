# UltraAPI

[![crates.io](https://img.shields.io/crates/v/ultraapi.svg)](https://crates.io/crates/ultraapi)
[![docs.rs](https://docs.rs/ultraapi/badge.svg)](https://docs.rs/ultraapi)

FastAPIに影響を受けたRustウェブフレームワークで、自动的なOpenAPI/Swaggerドキュメント生成をサポートしています。

## 特徴

- **自动OpenAPI生成**: すべてのルートが自動的にOpenAPI 3.1格式でドキュメント化されます
- **Swagger UI**: ビルトインの `/docs` エンドポイントがインタラクティブなAPIドキュメントを提供します
- **型安全**: Rustのコンパイル時チェックによる完全な型推論
- **依存性注入**: `Dep<T>`、`State<T>`、および `Depends<T>` エクストラクターの第一级サポート
- **Yield依赖**: FastAPIスタイルのジェネレーター依赖清理フックとスコープ管理（関数/リクエスト）
- **バリデーション**: `#[validate]` 属性によるビルトインvalidation（email、min/max length、pattern、数値範囲）
- **Router構成**: プレフィックス連結とtag/security伝播によるネストされたrouter
- **Resultハンドラー**: 適切なHTTPステータスコードを持つ `Result<T, ApiError>` の自动処理
- **Bearer Auth**: 簡単なJWT bearer認証セットアップ

## Quick Start

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Serialize, Deserialize, JsonSchema)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[post("/users")]
async fn create_user(user: User) -> User {
    User {
        id: 1,
        name: user.name,
        email: user.email,
    }
}

#[get("/users/{id}")]
async fn get_user(id: i64) -> Result<User, ApiError> {
    Ok(User {
        id,
        name: "Alice".into(),
        email: "alice@example.com".into(),
    })
}

#[derive(Clone)]
struct Database;

impl Database {
    async fn get_user(&self, id: i64) -> Option<User> {
        Some(User { id, name: "Alice".into(), email: "alice@example.com".into() })
    }
}

#[tokio::main]
async fn main() {
    let app = UltraApiApp::new()
        .title("My API")
        .version("1.0.0")
        .dep(Database)
        .route(create_user)
        .route(get_user);

    app.serve("0.0.0.0:3000").await;
}
```

## 主要マクロ

- `#[get(path)]` - GETエンドポイントを登録
- `#[post(path)]` - POSTエンドポイントを登録
- `#[put(path)]` - PUTエンドポイントを登録
- `#[delete(path)]` - DELETEエンドポイントを登録
- `#[api_model]` - struct/enumに対してvalidationとOpenAPIスキーマを生成
- `#[status(N)]` - ルートにカスタムHTTPステータスコードを設定
- `#[tag("name")]OpenAPIグループ化のtagを追加
- `#[security("scheme")]セキュリティschemeをルートに適用
- `#[response_class("json"|"html"|"text"|"binary"|"stream"|"xml")]` - レスポンスのcontent-typeを設定
- `#[summary("...")]` - OpenAPI summaryを設定
- `#[deprecated]` - ルートを非推奨としてマーク
- `#[external_docs(url = "...", description = "...")]` - 外部ドキュメントURLを設定

## Validation

UltraAPIは `#[api_model]` で定義されたstructに対して多种多様なvalidation属性をサポートしています:

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct User {
    /// メールアドレス
    #[validate(email)]
    email: String,
    
    /// ユーザー名（3-20文字）
    #[validate(min_length = 3)]
    #[validate(max_length = 20)]
    username: String,
    
    /// パスワード（8文字以上）
    #[validate(min_length = 8)]
    password: String,
    
    /// 年齢（0-150）
    #[validate(minimum = 0)]
    #[validate(maximum = 150)]
    age: i32,
    
    /// ユーザーID（英数字）
    #[validate(pattern = "^[a-zA-Z0-9_]+$")]
    user_id: String,
}
```

### 利用可能なValidation属性

- `#[validate(email)]` - メールアドレスとしてvalidation
- `#[validate(min_length = N)]` - 最小文字列長
- `#[validate(max_length = N)]` - 最大文字列長
- `#[validate(minimum = N)]` - 最小数値
- `#[validate(maximum = N)]` - 最大数値
- `#[validate(pattern = "regex")]` - パターンマッチ
- `#[validate(min_items = N)]` - 最小配列長

## Response Model Shaping

UltraAPIはFastAPIスタイルのresponse model shapingをサポートしています:

```rust
use ultraapi::prelude::*;

#[api_model]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct UserProfile {
    id: i64,
    username: String,
    email: String,
    password_hash: String,
    created_at: String,
    is_admin: bool,
}

// 特定のフィールドのみレスポンスに含める
#[get("/users/{id}/public", response_model(include={"id", "username"}))]
async fn get_public_profile(id: i64) -> UserProfile { ... }

// 敏感なフィールドをレスポンスから除外
#[get("/users/{id}/profile", response_model(exclude={"password_hash"}))]
async fn get_user_profile(id: i64) -> UserProfile { ... }

// alias名を使用（シリアライズ用）
#[get("/users/{id}/api", response_model(by_alias=true))]
async fn get_user_api(id: i64) -> UserProfile { ... }
```

## フィールド属性

- `#[alias("name")]` - シリアライズ時のフィールド名を変更
- `#[skip_serializing]` - シリアライズをスキップ
- `#[skip_deserializing]` - デシリアライズをスキップ
- `#[skip]` - シリアライズとデシリアライズの双方をスキップ
- `#[read_only]` - レスポンスのみに含める（リクエストボディには含まない）
- `#[write_only]` - リクエストのみに含める（レスポンスには含まない）

## 依存性の使い方

UltraAPIは3種類の依赖注入パターンをサポートしています:

### 1. `Dep<T>` - 简单的依赖注入

```rust
use ultraapi::prelude::*;

struct Database;

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[get("/users")]
async fn get_users(dep: Dep<Database>) -> String {
    // Database を使用
    "users".to_string()
}

#[tokio::main]
async fn main() {
    let app = UltraApiApp::new()
        .dep(Database)
        .route(get_users);
}
```

### 2. `State<T>` - 状态注入

```rust
use ultraapi::prelude::*;

#[derive(Clone)]
struct Config {
    api_key: String,
}

#[get("/config")]
async fn get_config(state: State<Config>) -> String {
    state.api_key.clone()
}
```

### 3. `Depends<T>` - FastAPIスタイルの依赖

```rust
use ultraapi::prelude::*;

struct CurrentUser {
    user_id: i64,
    username: String,
}

async fn get_current_user() -> CurrentUser {
    // 认证ロジック
    CurrentUser { user_id: 1, username: "alice".into() }
}

#[get("/profile")]
async fn get_profile(user: Depends<CurrentUser>) -> String {
    format!("Hello, {}!", user.username)
}
```

### Yield依赖（清理フック付き）

```rust
use ultraapi::prelude::*;
use std::sync::Arc;

struct DatabasePool { connection_string: String }

#[async_trait::async_trait]
impl Generator for DatabasePool {
    type Output = Self;
    type Error = DependencyError;

    async fn generate(self: Arc<Self>, _scope: Scope) -> Result<Self::Output, Self::Error> {
        Ok(Arc::try_unwrap(self).unwrap_or_else(|a| (*a).clone()))
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), Self::Error> {
        println!("データベース接続を閉じています");
        Ok(())
    }
}

// 関数スコープ（清理はレスポンスの前に実行）
let app = UltraApiApp::new()
    .yield_depends(Arc::new(DatabasePool { connection_string: "...".into() }), Scope::Function);

// リクエストスコープ（清理はレスポンスの後に実行）
let app = UltraApiApp::new()
    .yield_depends(Arc::new(DatabasePool { connection_string: "...".into() }), Scope::Request);
```

## OpenAPI / Swagger UI

UltraAPIは自动的にOpenAPI 3.1ドキュメントを生成します:

### エンドポイント

- `GET /openapi.json` - 生のOpenAPI 3.1仕様
- `GET /docs` - Swagger UI

### セキュリティscheme

```rust
use ultraapi::prelude::*;

// Bearer認証 (JWT)
let app = UltraApiApp::new()
    .bearer_auth();

// API Key認証
let app = UltraApiApp::new()
    .api_key("apiKeyAuth", "X-API-Key", "header");

// OAuth2 - Implicit Flow
let app = UltraApiApp::new()
    .oauth2_implicit(
        "oauth2Implicit",
        "https://example.com/authorize",
        [("read", "読み取りアクセス"), ("write", "書き込みアクセス")],
    );

// ルートにセキュリティを適用
#[get("/protected")]
#[security("oauth2Implicit")]
async fn protected_route() -> String {
    "秘密のデータ".to_string()
}
```

## Router

UltraAPIはネストされたrouterをサポートしています:

```rust
use ultraapi::prelude::*;

#[get("/hello")]
async fn hello() -> String {
    "Hello".to_string()
}

#[get("/world")]
async fn world() -> String {
    "World".to_string()
}

// ネストされたrouter
let api_router = Router::new()
    .route(hello)
    .route(world);

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .router("/api/v1", api_router);
```

## License

MIT
