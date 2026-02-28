# JWT（AuthLayer validator と統合）ガイド

UltraAPI では、FastAPI と同様に **Bearer トークン（JWT）** を `Authorization: Bearer <token>` で受け取り、ミドルウェア（AuthLayer）で検証できます。

このガイドでは「JWT を検証する `AuthValidator`」を実装して、`#[security("bearer")]` で保護されたルートを **実行時に 401/403 で制御**する方法を説明します。

> 注意
> - UltraAPI の `AuthValidator` は同期（sync）です。
> - JWT の署名検証（HMAC/RS256 など）は同期処理で完結するため、`AuthValidator` に適しています。
> - DB/Redis 参照が必要な場合は、検証結果キャッシュや別レイヤでの設計を検討してください。

---

## 1) OpenAPI（ドキュメント）側の bearerAuth を登録

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .bearer_auth();
```

---

## 2) ルート側で `#[security("bearer")]` を付ける

```rust
use ultraapi::prelude::*;

#[get("/me")]
#[security("bearer")]
async fn me() -> String {
    "secret".to_string()
}
```

---

## 3) AuthLayer（実行時 enforcement）を有効化する

`middleware(|builder| ...)` で AuthLayer を有効化し、
runtime の credential 抽出（Bearer）設定も追加します。

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::{SecuritySchemeConfig};

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .bearer_auth()
    .middleware(|builder| {
        builder
            .enable_auth() // まずは MockValidator で enforcement を有効化
            .with_security_scheme(SecuritySchemeConfig::bearer("bearerAuth"))
    });
```

本番では `enable_auth_with_validator(...)` を使って独自 validator を差し替えます。

---

## 4) JWT validator を実装する（例：jsonwebtoken を使用）

以下は **実運用向けの最小構成**の例です（HS256）。

```rust
use ultraapi::middleware::{AuthError, AuthValidator, Credentials};

// 例: jsonwebtoken を使う場合
// use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

#[derive(Clone)]
pub struct JwtValidator {
    // decoding_key: DecodingKey,
    // validation: Validation,
}

impl JwtValidator {
    pub fn new(/* secret: &str */) -> Self {
        // let mut validation = Validation::new(Algorithm::HS256);
        // validation.validate_exp = true;
        // Self { decoding_key: DecodingKey::from_secret(secret.as_bytes()), validation }
        Self {}
    }
}

impl AuthValidator for JwtValidator {
    fn validate(&self, credentials: &Credentials) -> Result<(), AuthError> {
        // scheme は "bearer" を想定
        if credentials.scheme.to_lowercase() != "bearer" {
            return Err(AuthError::unauthorized("Invalid auth scheme"));
        }

        let token = credentials.value.trim();
        if token.is_empty() {
            return Err(AuthError::unauthorized("Missing token"));
        }

        // 実際の JWT 検証（例）
        // let _data = decode::<Claims>(token, &self.decoding_key, &self.validation)
        //     .map_err(|_| AuthError::unauthorized("Invalid or expired token"))?;

        Ok(())
    }

    fn validate_scopes(
        &self,
        _credentials: &Credentials,
        required_scopes: &[String],
    ) -> Result<(), AuthError> {
        // 例: Claims の scopes/roles を照合して required_scopes を満たすかチェック
        // UltraAPI の実装では required_scopes は SecuritySchemeConfig から渡されます。
        if required_scopes.is_empty() {
            return Ok(());
        }

        // 必要に応じて 403 にする
        Err(AuthError::forbidden("Insufficient scope"))
    }
}
```

この `JwtValidator` をミドルウェアに登録します。

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::SecuritySchemeConfig;

let validator = JwtValidator::new();

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .bearer_auth()
    .middleware(|builder| {
        builder
            .enable_auth_with_validator(validator)
            .with_security_scheme(SecuritySchemeConfig::bearer("bearerAuth"))
    });
```

---

## 5) スコープ要件を追加する（任意）

`SecuritySchemeConfig::with_scopes(...)` で **要求スコープ**を指定できます。

```rust
use ultraapi::middleware::SecuritySchemeConfig;

let scheme = SecuritySchemeConfig::bearer("bearerAuth")
    .with_scopes(vec!["read".to_string()]);
```

---

## 参考

- `ultraapi/tests/security_tests.rs` に、runtime auth enforcement / scope validation の統合テストがあります
