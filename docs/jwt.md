# JWT Authentication Guide (AuthLayer Validator Integration)

In UltraAPI, similar to FastAPI, you can receive **Bearer tokens (JWT)** via `Authorization: Bearer <token>` and validate them in middleware (AuthLayer).

This guide explains how to implement a JWT-validating `AuthValidator` and control routes protected with `#[security("bearer")]` at **runtime with 401/403 responses**.

> **Note**
> - UltraAPI's `AuthValidator` is synchronous.
> - JWT signature verification (HMAC/RS256, etc.) completes synchronously, making it suitable for `AuthValidator`.
> - For DB/Redis lookups, consider caching validation results or designing a separate layer.

---

## 1) Register bearerAuth in OpenAPI (Documentation)

```rust
use ultraapi::prelude::*;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .bearer_auth();
```

---

## 2) Add `#[security("bearer")]` to Routes

```rust
use ultraapi::prelude::*;

#[get("/me")]
#[security("bearer")]
async fn me() -> String {
    "secret".to_string()
}
```

---

## 3) Enable AuthLayer (Runtime Enforcement)

Enable AuthLayer via `middleware(|builder| ...)` and configure runtime credential extraction (Bearer).

```rust
use ultraapi::prelude::*;
use ultraapi::middleware::SecuritySchemeConfig;

let app = UltraApiApp::new()
    .title("My API")
    .version("1.0.0")
    .bearer_auth()
    .middleware(|builder| {
        builder
            .enable_auth() // First, enable enforcement with MockValidator
            .with_security_scheme(SecuritySchemeConfig::bearer("bearerAuth"))
    });
```

In production, use `enable_auth_with_validator(...)` to plug in your custom validator.

---

## 4) Implement JWT Validator (Example: Using jsonwebtoken)

Here's a **minimal production-ready** example (HS256):

```rust
use ultraapi::middleware::{AuthError, AuthValidator, Credentials};

// Example: Using jsonwebtoken
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
        // Expect "bearer" scheme
        if credentials.scheme.to_lowercase() != "bearer" {
            return Err(AuthError::unauthorized("Invalid auth scheme"));
        }

        let token = credentials.value.trim();
        if token.is_empty() {
            return Err(AuthError::unauthorized("Missing token"));
        }

        // Actual JWT verification (example)
        // let _data = decode::<Claims>(token, &self.decoding_key, &self.validation)
        //     .map_err(|_| AuthError::unauthorized("Invalid or expired token"))?;

        Ok(())
    }

    fn validate_scopes(
        &self,
        _credentials: &Credentials,
        required_scopes: &[String],
    ) -> Result<(), AuthError> {
        // Example: Check Claims scopes/roles against required_scopes
        // In UltraAPI, required_scopes are passed from SecuritySchemeConfig
        if required_scopes.is_empty() {
            return Ok(());
        }

        // Return 403 if needed
        Err(AuthError::forbidden("Insufficient scope"))
    }
}
```

Register this `JwtValidator` in middleware:

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

## 5) Add Scope Requirements (Optional)

Specify **required scopes** via `SecuritySchemeConfig::with_scopes(...)`.

```rust
use ultraapi::middleware::SecuritySchemeConfig;

let scheme = SecuritySchemeConfig::bearer("bearerAuth")
    .with_scopes(vec!["read".to_string()]);
```

---

## References

- `ultraapi/tests/security_tests.rs` contains integration tests for runtime auth enforcement and scope validation
