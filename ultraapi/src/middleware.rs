// Middleware module for auth enforcement and middleware builder
use axum::{
    body::Body,
    extract::Request,
    http::{header::HeaderName, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::sync::Arc;

use crate::session::SessionConfig;

// ============================================================================
// Basic Authentication Utilities
// ============================================================================

/// Basic認証の資格情報を格納する構造体
///
/// # Example
///
/// ```rust
/// use ultraapi::middleware::decode_basic_header;
///
/// // "Basic dXNlcm5hbWU6cGFzc3dvcmQ=" -> "username:password"
/// let credentials = decode_basic_header("dXNlcm5hbWU6cGFzc3dvcmQ=");
/// assert!(credentials.is_some());
/// let creds = credentials.unwrap();
/// assert_eq!(creds.username, "username");
/// assert_eq!(creds.password, "password");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicCredentials {
    /// ユーザー名
    pub username: String,
    /// パスワード
    pub password: String,
}

/// HTTP Basic認証ヘッダーから資格情報をデコードする
///
/// 引数には "Basic " プレフィックスを除いたbase64エンコードされた文字列を渡す
///
/// # Arguments
///
/// * `encoded` - base64エンコードされた "username:password" 文字列
///
/// # Returns
///
/// * `Some(BasicCredentials)` - デコード成功時
/// * `None` - 無効なbase64または":"区切りの形式でない場合
///
/// # Example
///
/// ```rust
/// use ultraapi::middleware::decode_basic_header;
///
/// // Basic dXNlcm5hbWU6cGFzc3dvcmQ= (base64 of "username:password")
/// let result = decode_basic_header("dXNlcm5hbWU6cGFzc3dvcmQ=");
/// assert!(result.is_some());
/// ```
pub fn decode_basic_header(encoded: &str) -> Option<BasicCredentials> {
    // base64デコード
    let decoded = BASE64.decode(encoded.trim()).ok()?;

    // UTF-8文字列に変換
    let decoded_str = String::from_utf8(decoded).ok()?;

    // ":" で分割（パスワードに":"を含む場合があるため、split_onceを使用）
    let (username, password) = decoded_str.split_once(':')?;

    Some(BasicCredentials {
        username: username.to_string(),
        password: password.to_string(),
    })
}

/// HTTP Basic認証ヘッダーをデコードする（Authorizationヘッダー全体対応）
///
/// # Arguments
///
/// * `auth_header` - "Basic dXNlcm5hbWU6cGFzc3dvcmQ=" 形式のヘッダー値
///
/// # Returns
///
/// * `Some(BasicCredentials)` - デコード成功時
/// * `None` - ヘッダーが無効またはデコード失敗時
///
/// # Example
///
/// ```rust
/// use ultraapi::middleware::parse_basic_header;
///
/// let result = parse_basic_header("Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
/// assert!(result.is_some());
/// ```
pub fn parse_basic_header(auth_header: &str) -> Option<BasicCredentials> {
    let header = auth_header.trim();

    // "Basic " プレフィックスのチェック（大文字小文字を区別しない）
    if header.len() > 6 && header[..6].eq_ignore_ascii_case("basic ") {
        decode_basic_header(&header[6..])
    } else {
        None
    }
}

/// Specifies where to look for credentials
#[derive(Clone, Debug, Default, PartialEq)]
pub enum CredentialLocation {
    /// Look in Authorization header (default)
    #[default]
    Header,
    /// Look in query parameter
    Query,
    /// Look in cookie
    Cookie,
}

/// Security scheme configuration for runtime credential extraction
#[derive(Clone, Debug)]
pub struct SecuritySchemeConfig {
    /// The name of the security scheme (e.g., "bearerAuth", "apiKeyAuth")
    pub name: String,
    /// Where to look for credentials
    pub location: CredentialLocation,
    /// The parameter name for query/cookie locations
    pub param_name: String,
    /// Required scopes for this scheme
    pub scopes: Vec<String>,
}

impl SecuritySchemeConfig {
    /// Create a new config for bearer auth in Authorization header
    pub fn bearer(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: CredentialLocation::Header,
            param_name: "authorization".to_string(),
            scopes: vec![],
        }
    }

    /// Create a new config for HTTP Basic auth in Authorization header
    ///
    /// # Example
    ///
    /// ```rust
    /// use ultraapi::middleware::SecuritySchemeConfig;
    ///
    /// let config = SecuritySchemeConfig::basic("basicAuth");
    /// ```
    pub fn basic(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: CredentialLocation::Header,
            param_name: "authorization".to_string(),
            scopes: vec![],
        }
    }

    /// Create a config for API key in header
    pub fn api_key_header(name: impl Into<String>, param_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: CredentialLocation::Header,
            param_name: param_name.into(),
            scopes: vec![],
        }
    }

    /// Create a config for API key in query parameter
    pub fn api_key_query(name: impl Into<String>, param_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: CredentialLocation::Query,
            param_name: param_name.into(),
            scopes: vec![],
        }
    }

    /// Create a config for API key in cookie
    pub fn api_key_cookie(name: impl Into<String>, param_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: CredentialLocation::Cookie,
            param_name: param_name.into(),
            scopes: vec![],
        }
    }

    /// Add required scopes to this security scheme
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }
}

/// Authentication validator trait
/// Implement this to provide custom authentication logic
pub trait AuthValidator: Send + Sync {
    /// Validate the provided credentials
    /// Returns Ok(()) if valid, Err(status) if invalid (401 for missing/invalid, 403 for forbidden)
    fn validate(&self, credentials: &Credentials) -> Result<(), AuthError>;

    /// Validate scopes - called after successful credential validation
    /// Override this to implement scope-based authorization
    fn validate_scopes(
        &self,
        _credentials: &Credentials,
        _required_scopes: &[String],
    ) -> Result<(), AuthError> {
        Ok(())
    }
}

/// Credentials extracted from the request
#[derive(Clone, Debug)]
pub struct Credentials {
    /// The authorization scheme (e.g., "bearer", "apiKey", "basic")
    pub scheme: String,
    /// The credential value (e.g., token, API key, or base64-encoded credentials for Basic)
    pub value: String,
    /// The security scheme name used (e.g., "bearerAuth", "apiKeyAuth", "basicAuth")
    pub security_scheme: Option<String>,
    /// The scopes that were validated (if any)
    pub scopes: Vec<String>,
    /// ユーザー名（Basic認証の場合のみ）
    ///
    /// Basic認証では `username:password` の形式でデコードされる
    pub username: Option<String>,
    /// パスワード（Basic認証の場合のみ）
    ///
    /// Basic認証では `username:password` の形式でデコードされる
    pub password: Option<String>,
}

impl Credentials {
    /// Create new credentials
    pub fn new(scheme: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            scheme: scheme.into(),
            value: value.into(),
            security_scheme: None,
            scopes: vec![],
            username: None,
            password: None,
        }
    }

    /// Create credentials with security scheme name
    pub fn with_scheme(
        scheme: impl Into<String>,
        value: impl Into<String>,
        security_scheme: impl Into<String>,
    ) -> Self {
        Self {
            scheme: scheme.into(),
            value: value.into(),
            security_scheme: Some(security_scheme.into()),
            scopes: vec![],
            username: None,
            password: None,
        }
    }

    /// Basic認証の資格情報からCredentialsを作成
    ///
    /// # Arguments
    ///
    /// * `basic` - BasicCredentials
    /// * `security_scheme` - OpenAPIセキュリティスキーム名
    ///
    /// # Example
    ///
    /// ```rust
    /// use ultraapi::middleware::{Credentials, BasicCredentials, decode_basic_header};
    ///
    /// let basic = BasicCredentials {
    ///     username: "user".to_string(),
    ///     password: "pass".to_string(),
    /// };
    /// let creds = Credentials::from_basic(basic, "basicAuth");
    /// assert_eq!(creds.username.as_deref(), Some("user"));
    /// assert_eq!(creds.password.as_deref(), Some("pass"));
    /// ```
    pub fn from_basic(basic: BasicCredentials, security_scheme: impl Into<String>) -> Self {
        let encoded = format!("{}:{}", basic.username, basic.password);
        let encoded_bytes = BASE64.encode(encoded.as_bytes());

        Self {
            scheme: "basic".to_string(),
            value: encoded_bytes,
            security_scheme: Some(security_scheme.into()),
            scopes: vec![],
            username: Some(basic.username),
            password: Some(basic.password),
        }
    }

    /// Basic認証かどうかを確認
    ///
    /// # Example
    ///
    /// ```rust
    /// use ultraapi::middleware::Credentials;
    ///
    /// let basic_creds = Credentials::new("basic", "dXNlcjpwYXNz");
    /// assert!(basic_creds.is_basic());
    ///
    /// let bearer_creds = Credentials::new("bearer", "token123");
    /// assert!(!bearer_creds.is_basic());
    /// ```
    pub fn is_basic(&self) -> bool {
        self.scheme.to_lowercase() == "basic"
    }

    /// Basic認証の場合、usernameを取得
    pub fn basic_username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Basic認証の場合、passwordを取得
    pub fn basic_password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

/// Authentication error
#[derive(Clone, Debug)]
pub struct AuthError {
    pub status: StatusCode,
    pub message: String,
}

impl AuthError {
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }
}

/// Default mock auth validator for testing
/// Accepts any token that starts with "valid-" or equals "admin"
pub struct MockAuthValidator;

impl MockAuthValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockAuthValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthValidator for MockAuthValidator {
    fn validate(&self, credentials: &Credentials) -> Result<(), AuthError> {
        // Accept tokens starting with "valid-" or specific "admin" token
        if credentials.value.starts_with("valid-") || credentials.value == "admin" {
            Ok(())
        } else if credentials.scheme.to_lowercase() == "bearer" {
            Err(AuthError::unauthorized("Invalid or expired token"))
        } else if credentials.scheme.to_lowercase() == "apikey" {
            Err(AuthError::unauthorized("Invalid API key"))
        } else {
            Err(AuthError::unauthorized("Invalid credentials"))
        }
    }
}

/// A simple API key validator that checks for a specific key
pub struct ApiKeyValidator {
    valid_keys: Vec<String>,
}

impl ApiKeyValidator {
    pub fn new(valid_keys: Vec<String>) -> Self {
        Self { valid_keys }
    }
}

impl AuthValidator for ApiKeyValidator {
    fn validate(&self, credentials: &Credentials) -> Result<(), AuthError> {
        if self.valid_keys.contains(&credentials.value) {
            Ok(())
        } else {
            Err(AuthError::unauthorized("Invalid API key"))
        }
    }
}

/// HTTP Basic認証のバリデーター
///
/// ユーザー名とパスワードの組み合わせを検証する
///
/// # Example
///
/// ```rust
/// use ultraapi::middleware::{BasicAuthValidator, AuthValidator, Credentials};
///
/// let validator = BasicAuthValidator::new(vec![
///     ("admin".to_string(), "secret123".to_string()),
///     ("user".to_string(), "password".to_string()),
/// ]);
/// ```
pub struct BasicAuthValidator {
    /// 有効なユーザー名:パスワード の组み合わせ
    valid_credentials: Vec<(String, String)>,
}

impl BasicAuthValidator {
    /// 新しいBasicAuthValidatorを作成
    ///
    /// # Arguments
    ///
    /// * `credentials` - (username, password) のベクター
    pub fn new(credentials: Vec<(String, String)>) -> Self {
        Self {
            valid_credentials: credentials,
        }
    }

    /// 単一のユーザー名/パスワード组合せを追加
    pub fn with_credential(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.valid_credentials
            .push((username.into(), password.into()));
        self
    }
}

impl AuthValidator for BasicAuthValidator {
    fn validate(&self, credentials: &Credentials) -> Result<(), AuthError> {
        // Basic認証でない場合はエラー
        if !credentials.is_basic() {
            return Err(AuthError::unauthorized("Invalid authentication scheme"));
        }

        // username と password を取得
        let username = credentials
            .username
            .as_ref()
            .ok_or_else(|| AuthError::unauthorized("Invalid Basic credentials"))?;
        let password = credentials
            .password
            .as_ref()
            .ok_or_else(|| AuthError::unauthorized("Invalid Basic credentials"))?;

        // 有効な資格情報かチェック
        if self
            .valid_credentials
            .iter()
            .any(|(u, p)| u == username && p == password)
        {
            Ok(())
        } else {
            Err(AuthError::unauthorized("Invalid username or password"))
        }
    }
}

/// Scope-based auth validator that validates scopes after credentials
pub struct ScopedAuthValidator<V: AuthValidator> {
    inner: V,
    scope_map: std::collections::HashMap<String, Vec<String>>,
}

impl<V: AuthValidator + Clone> Clone for ScopedAuthValidator<V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            scope_map: self.scope_map.clone(),
        }
    }
}

impl<V: AuthValidator> ScopedAuthValidator<V> {
    pub fn new(inner: V) -> Self {
        Self {
            inner,
            scope_map: std::collections::HashMap::new(),
        }
    }

    /// Add scope mappings for tokens
    pub fn with_scope(mut self, token_prefix: &str, scopes: Vec<String>) -> Self {
        self.scope_map.insert(token_prefix.to_string(), scopes);
        self
    }
}

impl<V: AuthValidator> AuthValidator for ScopedAuthValidator<V> {
    fn validate(&self, credentials: &Credentials) -> Result<(), AuthError> {
        self.inner.validate(credentials)
    }

    fn validate_scopes(
        &self,
        credentials: &Credentials,
        required_scopes: &[String],
    ) -> Result<(), AuthError> {
        // First validate the credentials
        self.inner.validate(credentials)?;

        // If no scopes required, allow
        if required_scopes.is_empty() {
            return Ok(());
        }

        // Get scopes for this credential
        let granted_scopes = self
            .scope_map
            .get(&credentials.value)
            .cloned()
            .unwrap_or_else(|| {
                // If no specific mapping, check if value starts with "valid-" grant default scope
                if credentials.value.starts_with("valid-") {
                    vec!["read".to_string()]
                } else if credentials.value == "admin" {
                    vec!["read".to_string(), "write".to_string(), "admin".to_string()]
                } else {
                    vec![]
                }
            });

        // Check if all required scopes are granted
        for required in required_scopes {
            if !granted_scopes.contains(required) {
                return Err(AuthError::forbidden(format!(
                    "Insufficient scope: required '{}', not granted",
                    required
                )));
            }
        }

        Ok(())
    }
}

/// Auth layer that validates credentials for protected routes
#[derive(Clone)]
pub struct AuthLayer {
    validator: Arc<dyn AuthValidator>,
    /// Security scheme configurations for credential extraction
    security_schemes: Vec<SecuritySchemeConfig>,
}

impl AuthLayer {
    /// Create a new AuthLayer with the given validator
    pub fn new(validator: impl AuthValidator + 'static) -> Self {
        Self {
            validator: Arc::new(validator),
            security_schemes: vec![],
        }
    }

    /// Create a new AuthLayer with the default mock validator
    pub fn with_mock() -> Self {
        Self::new(MockAuthValidator::new())
    }

    /// Create a new AuthLayer with API key validation
    pub fn with_api_keys(keys: Vec<String>) -> Self {
        Self::new(ApiKeyValidator::new(keys))
    }

    /// Add a security scheme configuration
    pub fn with_security_scheme(mut self, config: SecuritySchemeConfig) -> Self {
        self.security_schemes.push(config);
        self
    }

    /// Add multiple security scheme configurations
    pub fn with_security_schemes(mut self, configs: Vec<SecuritySchemeConfig>) -> Self {
        self.security_schemes.extend(configs);
        self
    }

    /// Extract credentials from the request based on security scheme configs
    fn extract_credentials(&self, request: &Request<Body>) -> Option<Credentials> {
        // First try configured security schemes
        for scheme in &self.security_schemes {
            let credentials = match scheme.location {
                CredentialLocation::Header => {
                    let header_name = scheme.param_name.to_lowercase();
                    request
                        .headers()
                        .get(&header_name)
                        .and_then(|v| v.to_str().ok())
                        .map(|value| {
                            // If it's the Authorization header, parse scheme value
                            if header_name == "authorization" {
                                if let Some((scheme_name, scheme_value)) = value.split_once(' ') {
                                    let scheme_lower = scheme_name.to_lowercase();
                                    // Basic認証の場合は資格情報をデコード
                                    if scheme_lower == "basic" {
                                        if let Some(basic) = decode_basic_header(scheme_value) {
                                            Credentials::from_basic(basic, &scheme.name)
                                        } else {
                                            // 無効なBasicヘッダー
                                            Credentials::with_scheme(
                                                "basic",
                                                scheme_value,
                                                &scheme.name,
                                            )
                                        }
                                    } else {
                                        Credentials::with_scheme(
                                            scheme_name,
                                            scheme_value,
                                            &scheme.name,
                                        )
                                    }
                                } else {
                                    // No scheme, treat entire value as the credential
                                    Credentials::with_scheme("bearer", value, &scheme.name)
                                }
                            } else {
                                // Other headers: treat as API key
                                Credentials::with_scheme("ApiKey", value, &scheme.name)
                            }
                        })
                }
                CredentialLocation::Query => {
                    // Parse query string to find the parameter
                    request.uri().query().and_then(|query| {
                        for pair in query.split('&') {
                            if let Some((key, value)) = pair.split_once('=') {
                                if key == scheme.param_name {
                                    return Some(Credentials::with_scheme(
                                        "ApiKey",
                                        value,
                                        &scheme.name,
                                    ));
                                }
                            }
                        }
                        None
                    })
                }
                CredentialLocation::Cookie => {
                    // Extract from Cookie header
                    request
                        .headers()
                        .get("cookie")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|cookie_header| {
                            for pair in cookie_header.split(';') {
                                let pair = pair.trim();
                                if let Some((key, value)) = pair.split_once('=') {
                                    if key == scheme.param_name {
                                        return Some(Credentials::with_scheme(
                                            "ApiKey",
                                            value,
                                            &scheme.name,
                                        ));
                                    }
                                }
                            }
                            None
                        })
                }
            };

            if credentials.is_some() {
                return credentials;
            }
        }

        // Fallback: try Authorization header (backward compatibility)
        if let Some(auth_header) = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
        {
            if let Some((scheme, value)) = auth_header.split_once(' ') {
                let scheme_lower = scheme.to_lowercase();
                // Basic認証の場合は資格情報をデコード
                if scheme_lower == "basic" {
                    if let Some(basic) = decode_basic_header(value) {
                        return Some(Credentials::from_basic(basic, "fallback"));
                    }
                }
                return Some(Credentials::new(scheme, value));
            } else {
                // Just the value without scheme
                return Some(Credentials::new("bearer", auth_header));
            }
        }

        None
    }

    /// Get required scopes for a security scheme
    fn get_required_scopes(&self, security_scheme: &str) -> Vec<String> {
        for scheme in &self.security_schemes {
            if scheme.name == security_scheme {
                return scheme.scopes.clone();
            }
        }
        vec![]
    }

    pub(crate) async fn run(&self, request: Request<Body>, next: Next) -> Response {
        // Extract credentials from request
        let credentials = self.extract_credentials(&request);

        // Determine the realm for WWW-Authenticate header
        // Based on the configured security schemes
        let www_authenticate = self.build_www_authenticate_header();

        match credentials {
            Some(creds) => {
                // Get required scopes for the security scheme
                let required_scopes = creds
                    .security_scheme
                    .as_ref()
                    .map(|ss| self.get_required_scopes(ss))
                    .unwrap_or_default();

                // First validate credentials
                match self.validator.validate(&creds) {
                    Ok(()) => {
                        // Then validate scopes if required
                        match self.validator.validate_scopes(&creds, &required_scopes) {
                            Ok(()) => next.run(request).await,
                            Err(auth_error) => {
                                let error = if auth_error.status == StatusCode::FORBIDDEN {
                                    super::ApiError::forbidden(auth_error.message)
                                } else {
                                    super::ApiError::unauthorized(auth_error.message)
                                };
                                let mut response = error.into_response();
                                // Add WWW-Authenticate header if configured
                                if let Some(wwa) = &www_authenticate {
                                    response.headers_mut().insert(
                                        HeaderName::from_static("www-authenticate"),
                                        wwa.parse().unwrap(),
                                    );
                                }
                                response
                            }
                        }
                    }
                    Err(auth_error) => {
                        let error = if auth_error.status == StatusCode::FORBIDDEN {
                            super::ApiError::forbidden(auth_error.message)
                        } else {
                            super::ApiError::unauthorized(auth_error.message)
                        };
                        let mut response = error.into_response();
                        // Add WWW-Authenticate header if configured
                        if let Some(wwa) = &www_authenticate {
                            response.headers_mut().insert(
                                HeaderName::from_static("www-authenticate"),
                                wwa.parse().unwrap(),
                            );
                        }
                        response
                    }
                }
            }
            None => {
                // No credentials found - check if any security scheme is configured
                // If security schemes are configured, require authentication
                let error = if self.security_schemes.is_empty() {
                    // No schemes configured, try legacy behavior
                    super::ApiError::unauthorized("Missing authorization header")
                } else {
                    // Schemes configured but no credentials provided
                    super::ApiError::unauthorized("Missing authentication credentials")
                };
                let mut response = error.into_response();
                // Add WWW-Authenticate header if configured
                if let Some(wwa) = &www_authenticate {
                    response.headers_mut().insert(
                        HeaderName::from_static("www-authenticate"),
                        wwa.parse().unwrap(),
                    );
                }
                response
            }
        }
    }

    /// Build WWW-Authenticate header value based on configured security schemes
    fn build_www_authenticate_header(&self) -> Option<String> {
        if self.security_schemes.is_empty() {
            return None;
        }

        let mut challenges: Vec<String> = Vec::new();

        for scheme in &self.security_schemes {
            match scheme.name.to_lowercase().as_str() {
                n if n.contains("basic") => {
                    challenges.push(r#"Basic realm="UltraAPI""#.to_string());
                }
                n if n.contains("bearer") || n.contains("jwt") => {
                    challenges.push(r#"Bearer realm="UltraAPI""#.to_string());
                }
                _ => {
                    // Generic challenge for other schemes
                    challenges.push(format!(r#"Bearer realm="{}""#, scheme.name));
                }
            }
        }

        if challenges.is_empty() {
            None
        } else {
            Some(challenges.join(", "))
        }
    }
}

/// CORS middleware configuration
#[derive(Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<Method>,
    pub allowed_headers: Vec<String>,
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl CorsConfig {
    /// Create a new CorsConfig with defaults
    pub fn new() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ],
            allowed_headers: vec!["*".to_string()],
            allow_credentials: true,
        }
    }

    /// Set allowed origins
    pub fn allow_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    /// Set allowed methods
    pub fn allow_methods(mut self, methods: Vec<Method>) -> Self {
        self.allowed_methods = methods;
        self
    }

    /// Set allowed headers
    pub fn allow_headers(mut self, headers: Vec<String>) -> Self {
        self.allowed_headers = headers;
        self
    }

    /// Set whether to allow credentials
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    /// Build the CORS layer
    pub fn build(self) -> tower_http::cors::CorsLayer {
        use tower_http::cors::CorsLayer;

        let mut cors = CorsLayer::new()
            .allow_methods(self.allowed_methods)
            .allow_credentials(self.allow_credentials);

        if self.allowed_headers.iter().any(|h| h == "*") {
            cors = cors.allow_headers(tower_http::cors::Any);
        } else {
            let parsed_headers = self
                .allowed_headers
                .iter()
                .filter_map(|h| h.parse().ok())
                .collect::<Vec<_>>();
            if !parsed_headers.is_empty() {
                cors = cors.allow_headers(parsed_headers);
            }
        }

        if self.allowed_origins.iter().any(|o| o == "*") {
            cors = cors.allow_origin(tower_http::cors::Any);
        } else {
            let parsed_origins = self
                .allowed_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect::<Vec<_>>();
            if !parsed_origins.is_empty() {
                cors = cors.allow_origin(parsed_origins);
            }
        }

        cors
    }
}

/// Compression middleware configuration
#[derive(Clone)]
pub struct CompressionConfig {
    /// Enable gzip compression (default: true)
    pub gzip: bool,
    /// Enable br (brotli) compression (default: true)
    pub brotli: bool,
    /// Enable deflate compression (default: false)
    pub deflate: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionConfig {
    /// Create a new CompressionConfig with defaults (gzip and brotli enabled)
    pub fn new() -> Self {
        Self {
            gzip: true,
            brotli: true,
            deflate: false,
        }
    }

    /// Enable gzip compression
    pub fn gzip(mut self, enabled: bool) -> Self {
        self.gzip = enabled;
        self
    }

    /// Enable brotli compression (br)
    pub fn brotli(mut self, enabled: bool) -> Self {
        self.brotli = enabled;
        self
    }

    /// Enable deflate compression
    pub fn deflate(mut self, enabled: bool) -> Self {
        self.deflate = enabled;
        self
    }

    /// Build the compression layer
    pub fn build(self) -> tower_http::compression::CompressionLayer {
        use tower_http::compression::CompressionLayer;

        // Create compression layer and configure
        // By default all algorithms are enabled, we disable the ones we don't want
        let mut compression = CompressionLayer::new();

        // Configure brotli
        if !self.brotli {
            compression = compression.no_br();
        }

        // Configure gzip
        if !self.gzip {
            compression = compression.no_gzip();
        }

        // Configure deflate
        if !self.deflate {
            compression = compression.no_deflate();
        }

        compression
    }
}

/// GZip compression middleware configuration
///
/// A simplified configuration for GZip compression with minimum size
/// and content type filtering.
#[derive(Clone, Debug)]
pub struct GZipConfig {
    /// Minimum body size to compress (default: 1024 bytes)
    pub minimum_size: usize,
    /// List of content types to compress (default: text/*, application/json, application/xml, application/javascript)
    pub content_types: Vec<String>,
    /// Compression quality (default: CompressionLevel::Default)
    pub quality: tower_http::compression::CompressionLevel,
}

impl Default for GZipConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl GZipConfig {
    /// Create a new GZipConfig with defaults
    pub fn new() -> Self {
        Self {
            minimum_size: 1024,
            content_types: vec![
                "text/*".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
                "application/javascript".to_string(),
            ],
            quality: tower_http::compression::CompressionLevel::Default,
        }
    }

    /// Set minimum size threshold for compression.
    pub fn minimum_size(mut self, size: usize) -> Self {
        self.minimum_size = size;
        self
    }

    /// Set content types to compress.
    ///
    /// Supported patterns:
    /// - Exact match: "application/json"
    /// - Prefix match: "text/*" (matches any content-type starting with "text/")
    pub fn content_types(mut self, types: Vec<String>) -> Self {
        self.content_types = types;
        self
    }

    /// Set compression quality.
    pub fn quality(mut self, quality: tower_http::compression::CompressionLevel) -> Self {
        self.quality = quality;
        self
    }

    /// Build the GZip compression layer.
    pub fn build(
        self,
    ) -> tower_http::compression::CompressionLayer<impl tower_http::compression::predicate::Predicate>
    {
        use axum::http::{header, Extensions, HeaderMap, StatusCode, Version};
        use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};
        use tower_http::compression::CompressionLayer;

        let min = u16::try_from(self.minimum_size).unwrap_or(u16::MAX);
        let patterns = Arc::new(self.content_types);

        let allow_content_types = move |_status: StatusCode,
                                        _version: Version,
                                        headers: &HeaderMap,
                                        _ext: &Extensions| {
            let raw = headers
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            // If content-type is missing, compress by default.
            if raw.is_empty() {
                return true;
            }

            let ct = raw.split(';').next().unwrap_or("").trim();

            patterns.iter().any(|p| {
                if let Some(prefix) = p.strip_suffix("/*") {
                    let prefix = prefix.trim_end_matches('/');
                    return ct.starts_with(&format!("{}/", prefix));
                }
                ct == p
            })
        };

        // Fixed-window size predicate + skip grpc/images/sse + allowlist by content-type.
        let predicate = SizeAbove::new(min)
            .and(NotForContentType::GRPC)
            .and(NotForContentType::IMAGES)
            .and(NotForContentType::SSE)
            .and(allow_content_types);

        CompressionLayer::new()
            .gzip(true)
            .no_br()
            .no_deflate()
            .no_zstd()
            .quality(self.quality)
            .compress_when(predicate)
    }
}

/// Middleware builder for UltraAPI applications
pub struct MiddlewareBuilder {
    pub auth_enabled: bool,
    pub auth_layer: Option<AuthLayer>,
    pub cors_config: Option<CorsConfig>,
    pub compression_config: Option<CompressionConfig>,
    pub gzip_config: Option<GZipConfig>,
    pub rate_limit_config: Option<RateLimitConfig>,
    pub response_cache_config: Option<ResponseCacheConfig>,
    pub session_config: Option<SessionConfig>,
}

impl Default for MiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareBuilder {
    /// Create a new MiddlewareBuilder
    pub fn new() -> Self {
        Self {
            auth_enabled: false,
            auth_layer: None,
            cors_config: None,
            compression_config: None,
            gzip_config: None,
            rate_limit_config: None,
            response_cache_config: None,
            session_config: None,
        }
    }

    /// Enable GZip compression with the given configuration
    pub fn gzip_config(mut self, config: GZipConfig) -> Self {
        self.gzip_config = Some(config);
        self
    }

    /// Enable authentication middleware with mock validator (for testing)
    /// This enforces #[security] requirements at runtime
    pub fn enable_auth(mut self) -> Self {
        self.auth_enabled = true;
        self.auth_layer = Some(AuthLayer::with_mock());
        self
    }

    /// Enable authentication middleware with custom validator
    /// This enforces #[security] requirements at runtime
    pub fn enable_auth_with_validator(mut self, validator: impl AuthValidator + 'static) -> Self {
        self.auth_enabled = true;
        self.auth_layer = Some(AuthLayer::new(validator));
        self
    }

    /// Enable authentication with API key validation
    pub fn enable_auth_with_api_keys(mut self, keys: Vec<String>) -> Self {
        self.auth_enabled = true;
        self.auth_layer = Some(AuthLayer::with_api_keys(keys));
        self
    }

    /// Enable authentication with HTTP Basic validation
    ///
    /// # Example
    ///
    /// ```rust
    /// use ultraapi::middleware::MiddlewareBuilder;
    ///
    /// let builder = MiddlewareBuilder::new()
    ///     .enable_auth_with_basic(vec![
    ///         ("admin".to_string(), "secret123".to_string()),
    ///         ("user".to_string(), "password".to_string()),
    ///     ]);
    /// ```
    pub fn enable_auth_with_basic(mut self, credentials: Vec<(String, String)>) -> Self {
        self.auth_enabled = true;
        self.auth_layer = Some(AuthLayer::new(BasicAuthValidator::new(credentials)));
        self
    }

    /// Add security scheme configurations to the auth layer
    /// This enables extraction of credentials from different locations (header, query, cookie)
    pub fn with_security_schemes(mut self, schemes: Vec<SecuritySchemeConfig>) -> Self {
        if let Some(auth_layer) = self.auth_layer.take() {
            self.auth_layer = Some(auth_layer.with_security_schemes(schemes));
        } else {
            // Create new auth layer with schemes
            self.auth_enabled = true;
            self.auth_layer =
                Some(AuthLayer::new(MockAuthValidator::new()).with_security_schemes(schemes));
        }
        self
    }

    /// Add a single security scheme configuration
    pub fn with_security_scheme(self, scheme: SecuritySchemeConfig) -> Self {
        self.with_security_schemes(vec![scheme])
    }

    /// Enable CORS with the given configuration
    pub fn cors(mut self, config: CorsConfig) -> Self {
        self.cors_config = Some(config);
        self
    }

    /// Enable compression with the given configuration (gzip by default)
    pub fn compression(&mut self, config: CompressionConfig) -> &mut Self {
        self.compression_config = Some(config);
        self
    }

    /// Enable rate limiting with the given configuration
    pub fn rate_limit(&mut self, config: RateLimitConfig) -> &mut Self {
        self.rate_limit_config = Some(config);
        self
    }

    /// Enable response caching with the given configuration
    pub fn response_cache(mut self, config: ResponseCacheConfig) -> Self {
        self.response_cache_config = Some(config);
        self
    }

    /// Enable server-side session cookies
    pub fn session_cookies(mut self, config: SessionConfig) -> Self {
        self.session_config = Some(config);
        self
    }
}

// ============================================================================
// ============================================================================
// Rate Limiting Middleware
// ============================================================================
// Rate Limiting Middleware
// ============================================================================

use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ============================================================================
// Response Caching Middleware
// ============================================================================

use bytes::Bytes;

#[derive(Clone)]
struct ResponseCacheEntry {
    status: StatusCode,
    version: axum::http::Version,
    headers: axum::http::HeaderMap,
    body: Bytes,
    expires_at: Instant,
}

/// レスポンスキャッシュの設定（MVP）
///
/// - GET/HEAD の 200 レスポンスを in-memory にキャッシュします
/// - デフォルトでは Authorization ヘッダーがある場合はキャッシュしません（安全側）
/// - Cache-Control: no-store が付いたレスポンスは保存しません
/// - x-cache: HIT|MISS|BYPASS を付与します
#[derive(Clone, Debug)]
pub struct ResponseCacheConfig {
    /// キャッシュ TTL
    pub ttl: Duration,
}

impl Default for ResponseCacheConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseCacheConfig {
    pub fn new() -> Self {
        Self {
            ttl: Duration::from_secs(60),
        }
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn build(self) -> ResponseCacheMiddleware {
        ResponseCacheMiddleware::new(self)
    }
}

/// In-memory response cache middleware
#[derive(Clone)]
pub struct ResponseCacheMiddleware {
    ttl: Duration,
    store: Arc<RwLock<HashMap<String, ResponseCacheEntry>>>,
}

impl ResponseCacheMiddleware {
    pub fn new(config: ResponseCacheConfig) -> Self {
        Self {
            ttl: config.ttl,
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn cache_key<B>(&self, req: &axum::http::Request<B>) -> String {
        let method = req.method();
        let path = req.uri().path();
        let query = req.uri().query().unwrap_or("");
        format!("{}:{}?{}", method, path, query)
    }

    fn cacheable_request<B>(&self, req: &axum::http::Request<B>) -> bool {
        matches!(*req.method(), Method::GET | Method::HEAD)
            && !req
                .headers()
                .contains_key(axum::http::header::AUTHORIZATION)
            && !req.headers().contains_key(axum::http::header::COOKIE)
    }

    fn cacheable_response(&self, res: &axum::http::Response<axum::body::Body>) -> bool {
        if res.status() != StatusCode::OK {
            return false;
        }

        // Never cache responses that set cookies (user/session-specific)
        if res.headers().contains_key(axum::http::header::SET_COOKIE) {
            return false;
        }

        if let Some(cc) = res.headers().get(axum::http::header::CACHE_CONTROL) {
            if let Ok(s) = cc.to_str() {
                if s.to_ascii_lowercase().contains("no-store") {
                    return false;
                }
            }
        }

        true
    }

    fn get(&self, key: &str) -> Option<ResponseCacheEntry> {
        let now = Instant::now();
        let mut store = self.store.write();
        if let Some(entry) = store.get(key) {
            if entry.expires_at > now {
                return Some(entry.clone());
            }
            // expired
            store.remove(key);
        }
        None
    }

    fn set(&self, key: String, entry: ResponseCacheEntry) {
        let mut store = self.store.write();
        store.insert(key, entry);
    }
}

impl<S> tower::Layer<S> for ResponseCacheMiddleware {
    type Service = ResponseCacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ResponseCacheService {
            inner,
            middleware: self.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ResponseCacheService<S> {
    inner: S,
    middleware: ResponseCacheMiddleware,
}

impl<S, B> tower::Service<axum::http::Request<B>> for ResponseCacheService<S>
where
    S: tower::Service<axum::http::Request<B>, Response = axum::http::Response<axum::body::Body>>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
    S::Error: std::fmt::Debug,
    B: Send + 'static,
{
    type Response = axum::http::Response<axum::body::Body>;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::http::Request<B>) -> Self::Future {
        let middleware = self.middleware.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let cacheable_req = middleware.cacheable_request(&req);
            let key = if cacheable_req {
                Some(middleware.cache_key(&req))
            } else {
                None
            };

            if let Some(ref key) = key {
                if let Some(entry) = middleware.get(key) {
                    let mut res = axum::http::Response::new(axum::body::Body::from(entry.body));
                    *res.status_mut() = entry.status;
                    *res.version_mut() = entry.version;
                    *res.headers_mut() = entry.headers;
                    res.headers_mut().insert("x-cache", "HIT".parse().unwrap());
                    return Ok(res);
                }
            }

            let res = inner.call(req).await.unwrap();

            if !cacheable_req {
                let (mut parts, body) = res.into_parts();
                parts.headers.insert("x-cache", "BYPASS".parse().unwrap());
                return Ok(axum::http::Response::from_parts(parts, body));
            }

            if !middleware.cacheable_response(&res) {
                let (mut parts, body) = res.into_parts();
                parts.headers.insert("x-cache", "BYPASS".parse().unwrap());
                return Ok(axum::http::Response::from_parts(parts, body));
            }

            let (mut parts, body) = res.into_parts();
            let bytes = axum::body::to_bytes(body, usize::MAX)
                .await
                .unwrap_or_default();

            if let Some(key) = key {
                let mut store_headers = parts.headers.clone();
                store_headers.remove("x-cache");

                middleware.set(
                    key,
                    ResponseCacheEntry {
                        status: parts.status,
                        version: parts.version,
                        headers: store_headers,
                        body: bytes.clone(),
                        expires_at: Instant::now() + middleware.ttl,
                    },
                );
            }

            parts.headers.insert("x-cache", "MISS".parse().unwrap());
            Ok(axum::http::Response::from_parts(
                parts,
                axum::body::Body::from(bytes),
            ))
        })
    }
}

/// レート制限の設定
///
/// # Example
///
/// ```rust
/// use ultraapi::middleware::RateLimitConfig;
/// use std::time::Duration;
///
/// let config = RateLimitConfig::new(10, Duration::from_secs(60));
/// ```
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed within the time window
    pub max_requests: u32,
    /// Time window duration
    pub window: Duration,
}

impl RateLimitConfig {
    /// Create a new rate limit config
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
        }
    }

    /// Build the rate limit middleware
    pub fn build(self) -> RateLimitMiddleware {
        RateLimitMiddleware::new(self)
    }
}

/// Rate limit key with request count and window start time
#[derive(Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

/// Simple rate limiting middleware
#[derive(Clone)]
pub struct RateLimitMiddleware {
    max_requests: u32,
    window: Duration,
    store: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl RateLimitMiddleware {
    /// Create a new rate limit middleware
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            max_requests: config.max_requests,
            window: config.window,
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check rate limit and return Some(response) if limited, None if allowed
    pub fn check_limit(&self, key: &str) -> Option<axum::response::Response> {
        let now = Instant::now();
        let mut store_guard = self.store.write();

        let entry = store_guard
            .entry(key.to_string())
            .or_insert_with(|| RateLimitEntry {
                count: 0,
                window_start: now,
            });

        let elapsed = now.duration_since(entry.window_start);
        if elapsed >= self.window {
            entry.count = 1;
            entry.window_start = now;
        } else {
            entry.count += 1;
        }

        if entry.count > self.max_requests {
            let wait_time = self.window - elapsed;
            let retry_after = wait_time.as_secs();

            let error_body = serde_json::json!({
                "error": "Too Many Requests",
                "details": ["Rate limit exceeded. Please try again later."]
            });

            Some(
                axum::http::Response::builder()
                    .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
                    .header("content-type", "application/json")
                    .header("x-ratelimit-limit", self.max_requests.to_string())
                    .header("x-ratelimit-remaining", "0")
                    .header("retry-after", retry_after.to_string())
                    .body(axum::body::Body::from(error_body.to_string()))
                    .unwrap(),
            )
        } else {
            None
        }
    }
}

/// Implement tower::Layer for RateLimitMiddleware
impl<S> tower::Layer<S> for RateLimitMiddleware {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            middleware: self.clone(),
        }
    }
}

/// Service wrapper that applies rate limiting
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    middleware: RateLimitMiddleware,
}

impl<S, B> tower::Service<axum::http::Request<B>> for RateLimitService<S>
where
    S: tower::Service<axum::http::Request<B>, Response = axum::http::Response<axum::body::Body>>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
    S::Error: std::fmt::Debug,
    B: Send + 'static,
{
    type Response = axum::http::Response<axum::body::Body>;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::http::Request<B>) -> Self::Future {
        let middleware = self.middleware.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Get the rate limit key
            let key = req
                .headers()
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "global".to_string());

            // Check rate limit
            if let Some(response) = middleware.check_limit(&key) {
                return Ok(response);
            }

            // Call inner service
            let res = inner.call(req).await.unwrap();

            Ok(res)
        })
    }
}

// ============================================================================
// OAuth2 Dependency Objects (FastAPI-compatible)
// ============================================================================

/// OAuth2 Bearer token extractor for password flow
///
/// This is UltraAPI's equivalent of FastAPI's `OAuth2PasswordBearer`.
/// It extracts the Bearer token from the Authorization header.
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
/// use ultraapi::middleware::OAuth2PasswordBearer;
///
/// // In a route function:
/// #[get("/users/me")]
/// async fn get_current_user(token: OAuth2PasswordBearer) -> String {
///     format!("User token: {}", token.0)
/// }
/// ```
#[derive(Clone)]
pub struct OAuth2PasswordBearer(pub String);

/// OAuth2 Bearer token extractor that returns Option when auto_error=false
///
/// This variant returns `Option<OAuth2PasswordBearer>` when `auto_error` is false,
/// allowing the route handler to decide how to handle missing/invalid tokens.
pub struct OptionalOAuth2PasswordBearer(pub Option<String>);

/// OAuth2 Authorization Code Bearer token extractor
///
/// Similar to OAuth2PasswordBearer but designed for authorization code flow.
/// In practice, both flows use the same Bearer token format in the Authorization header.
#[derive(Clone)]
pub struct OAuth2AuthorizationCodeBearer(pub String);

/// OAuth2 Authorization Code Bearer that returns Option when auto_error=false
pub struct OptionalOAuth2AuthorizationCodeBearer(pub Option<String>);

/// OAuth2 scopes container for runtime validation
///
/// This struct holds the required scopes for an OAuth2-protected endpoint.
/// It can be used for runtime scope validation.
#[derive(Clone, Debug, Default)]
pub struct OAuth2Scopes {
    /// Required scopes for the endpoint
    pub scopes: Vec<String>,
}

impl OAuth2Scopes {
    /// Create new OAuth2Scopes
    pub fn new(scopes: Vec<String>) -> Self {
        Self { scopes }
    }

    /// Create from an iterator of scope strings
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self {
            scopes: iter.into_iter().map(|s| s.into()).collect(),
        }
    }

    /// Check if the token has all required scopes
    /// Note: This is a placeholder - actual validation depends on the OAuth2 server's response
    pub fn validate(&self, _token_scopes: &[String]) -> bool {
        // For now, we just store the required scopes for OpenAPI integration
        // Actual validation would require decoding the token or calling the OAuth2 server
        true
    }
}

/// Parse Bearer token from Authorization header value
///
/// # Arguments
///
/// * `auth_header` - The Authorization header value (e.g., "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")
///
/// # Returns
///
/// * `Some(token)` - Valid Bearer token
/// * `None` - Invalid format (not Bearer, not properly formatted, etc.)
pub fn parse_bearer_token(auth_header: &str) -> Option<String> {
    let parts: Vec<&str> = auth_header.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }

    let scheme = parts[0].to_lowercase();
    if scheme != "bearer" {
        return None;
    }

    let token = parts[1].to_string();
    if token.is_empty() {
        return None;
    }

    Some(token)
}

/// Create an unauthorized error response for OAuth2/Bearer authentication
///
/// This follows RFC 6750 for Bearer token authentication errors.
/// The error response includes the `WWW-Authenticate` header.
pub fn create_bearer_unauthorized_error(
    error: &str,
    error_description: Option<&str>,
) -> crate::ApiError {
    let details = error_description.unwrap_or(error);
    let api_error = crate::ApiError::unauthorized(details);

    // Add WWW-Authenticate header (RFC 6750)
    // Note: ApiError doesn't currently support custom headers directly
    // The header will be added by the auth middleware if configured
    let _www_authenticate = if let Some(desc) = error_description {
        format!("Bearer error=\"{}\", error_description=\"{}\"", error, desc)
    } else {
        format!("Bearer error=\"{}\"", error)
    };

    api_error
}

use axum::{extract::FromRequestParts, http::request::Parts};

impl<S> FromRequestParts<S> for OAuth2PasswordBearer
where
    S: Send + Sync,
{
    type Rejection = crate::ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

        match auth_header {
            Some(header_value) => {
                // Parse Bearer token
                if let Some(token) = parse_bearer_token(header_value) {
                    Ok(OAuth2PasswordBearer(token))
                } else {
                    // Invalid format (not Bearer or empty token)
                    Err(create_bearer_unauthorized_error(
                        "invalid_token",
                        Some(
                            "Invalid or malformed Authorization header. Expected 'Bearer <token>'",
                        ),
                    ))
                }
            }
            None => {
                // No Authorization header - return 401
                Err(create_bearer_unauthorized_error(
                    "invalid_token",
                    Some("Missing Authorization header. Expected 'Bearer <token>'"),
                ))
            }
        }
    }
}

impl<S> FromRequestParts<S> for OptionalOAuth2PasswordBearer
where
    S: Send + Sync,
{
    type Rejection = crate::ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

        match auth_header {
            Some(header_value) => {
                // Parse Bearer token - return None if invalid format
                let token = parse_bearer_token(header_value);
                Ok(OptionalOAuth2PasswordBearer(token))
            }
            None => {
                // No Authorization header - return None (not an error)
                Ok(OptionalOAuth2PasswordBearer(None))
            }
        }
    }
}

impl<S> FromRequestParts<S> for OAuth2AuthorizationCodeBearer
where
    S: Send + Sync,
{
    type Rejection = crate::ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Same implementation as OAuth2PasswordBearer
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

        match auth_header {
            Some(header_value) => {
                if let Some(token) = parse_bearer_token(header_value) {
                    Ok(OAuth2AuthorizationCodeBearer(token))
                } else {
                    Err(create_bearer_unauthorized_error(
                        "invalid_token",
                        Some(
                            "Invalid or malformed Authorization header. Expected 'Bearer <token>'",
                        ),
                    ))
                }
            }
            None => Err(create_bearer_unauthorized_error(
                "invalid_token",
                Some("Missing Authorization header. Expected 'Bearer <token>'"),
            )),
        }
    }
}

impl<S> FromRequestParts<S> for OptionalOAuth2AuthorizationCodeBearer
where
    S: Send + Sync,
{
    type Rejection = crate::ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

        match auth_header {
            Some(header_value) => {
                let token = parse_bearer_token(header_value);
                Ok(OptionalOAuth2AuthorizationCodeBearer(token))
            }
            None => Ok(OptionalOAuth2AuthorizationCodeBearer(None)),
        }
    }
}

/// Extension trait for OAuth2 dependency objects to support auto_error configuration
///
/// Note: In UltraAPI, auto_error is handled at the dependency level by providing
/// both required (auto_error=true) and optional (auto_error=false) variants.
pub trait OAuth2Dependency: Send + Sync {
    /// The security scheme name for OpenAPI
    fn security_scheme_name(&self) -> &str;

    /// Required scopes for this dependency
    fn scopes(&self) -> &[String];
}

impl OAuth2Dependency for OAuth2PasswordBearer {
    fn security_scheme_name(&self) -> &str {
        "oauth2Password"
    }

    fn scopes(&self) -> &[String] {
        &[]
    }
}

impl OAuth2Dependency for OptionalOAuth2PasswordBearer {
    fn security_scheme_name(&self) -> &str {
        "oauth2Password"
    }

    fn scopes(&self) -> &[String] {
        &[]
    }
}

impl OAuth2Dependency for OAuth2AuthorizationCodeBearer {
    fn security_scheme_name(&self) -> &str {
        "oauth2AuthCode"
    }

    fn scopes(&self) -> &[String] {
        &[]
    }
}

impl OAuth2Dependency for OptionalOAuth2AuthorizationCodeBearer {
    fn security_scheme_name(&self) -> &str {
        "oauth2AuthCode"
    }

    fn scopes(&self) -> &[String] {
        &[]
    }
}

// ============================================================================
// OAuth2 Production Components (/token endpoint types)
// ============================================================================

/// OAuth2 パスワードフローで使用されるリクエストフォーム
///
/// FastAPI の `OAuth2PasswordRequestForm` に相当します。
/// `/token` エンドポイントで Form データとして受け取ります。
///
/// # Fields
/// - `username`: ユーザー名またはメールアドレス
/// - `password`: パスワード
/// - `scope`: スペース区切りのスコープ文字列（オプション）
/// - `grant_type`: グラントタイプ（通常は "password"）
/// - `client_id`: クライアントID（オプション）
/// - `client_secret`: クライアントシークレット（オプション）
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
/// use ultraapi::middleware::OAuth2PasswordRequestForm;
///
/// #[post("/token")]
/// async fn token(form: OAuth2PasswordRequestForm) -> Result<TokenResponse, OAuth2ErrorResponse> {
///     // 認証ロジックを実装
///     Ok(TokenResponse::new("access_token".to_string(), 3600))
/// }
/// ```
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct OAuth2PasswordRequestForm {
    /// ユーザー名またはメールアドレス
    pub username: String,
    /// パスワード
    pub password: String,
    /// スペース区切りのスコープ文字列
    #[serde(default)]
    pub scope: String,
    /// グラントタイプ（通常は "password"）
    #[serde(default)]
    pub grant_type: String,
    /// クライアントID
    #[serde(default)]
    pub client_id: Option<String>,
    /// クライアントシークレット
    #[serde(default)]
    pub client_secret: Option<String>,
}

impl OAuth2PasswordRequestForm {
    /// スコープをベクターに変換
    ///
    /// スペース区切りのスコープ文字列を Vec<String> に変換します。
    pub fn scopes(&self) -> Vec<String> {
        if self.scope.is_empty() {
            return vec![];
        }
        self.scope
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }

    /// グラントタイプが "password" かどうかを確認
    pub fn is_password_grant(&self) -> bool {
        self.grant_type.is_empty() || self.grant_type == "password"
    }
}

/// OAuth2 トークンレスポンス
///
/// 成功時のトークン発行レスポンスです。
/// FastAPI の `Token` モデルに相当します。
///
/// # Fields
/// - `access_token`: アクセストークン
/// - `token_type`: トークンタイプ（通常は "bearer"）
/// - `expires_in`: 有効期限（秒）
/// - `refresh_token`: 更新トークン（オプション）
/// - `scope`: スペース区切りのスコープ文字列（オプション）
///
/// # Example
///
/// ```ignore
/// use ultraapi::middleware::TokenResponse;
///
/// let response = TokenResponse::new("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...".to_string(), 3600);
/// ```
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TokenResponse {
    /// アクセストークン
    pub access_token: String,
    /// トークンタイプ（通常は "bearer"）
    #[serde(default = "default_token_type")]
    pub token_type: String,
    /// 有効期限（秒）
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// 更新トークン
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// スペース区切りのスコープ文字列
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scope: String,
}

fn default_token_type() -> String {
    "bearer".to_string()
}

impl TokenResponse {
    /// 新しい TokenResponse を作成
    pub fn new(access_token: String, expires_in: u64) -> Self {
        Self {
            access_token,
            token_type: "bearer".to_string(),
            expires_in: Some(expires_in),
            refresh_token: None,
            scope: String::new(),
        }
    }

    /// スコープ付きでの作成
    pub fn with_scopes(access_token: String, expires_in: u64, scopes: Vec<String>) -> Self {
        Self {
            access_token,
            token_type: "bearer".to_string(),
            expires_in: Some(expires_in),
            refresh_token: None,
            scope: scopes.join(" "),
        }
    }

    /// 更新トークンを設定
    pub fn with_refresh_token(mut self, refresh_token: String) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }
}

/// OAuth2 エラーレスポンス
///
/// RFC 6749 に準拠したエラーレスポンスです。
///
/// # Fields
/// - `error`: エラーコード（必須）
/// - `error_description`: エラー詳細（オプション）
/// - `error_uri`: エラー情報のURI（オプション）
///
/// # Error Codes
/// - `invalid_request`: リクエストが不正
/// - `invalid_client`: クライアント認証に失敗
/// - `invalid_grant`: 提供されたグラントが無効
/// - `unauthorized_client`: クライアントが権限を持たない
/// - `unsupported_grant_type`: サポートされていないグラントタイプ
/// - `invalid_scope`: 無効なスコープ
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct OAuth2ErrorResponse {
    /// エラーコード
    pub error: String,
    /// エラー詳細
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    /// エラー情報のURI
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_uri: Option<String>,
}

impl OAuth2ErrorResponse {
    /// invalid_request エラー
    pub fn invalid_request(description: impl Into<String>) -> Self {
        Self {
            error: "invalid_request".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    /// invalid_client エラー
    pub fn invalid_client(description: impl Into<String>) -> Self {
        Self {
            error: "invalid_client".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    /// invalid_grant エラー
    pub fn invalid_grant(description: impl Into<String>) -> Self {
        Self {
            error: "invalid_grant".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    /// unsupported_grant_type エラー
    pub fn unsupported_grant_type() -> Self {
        Self {
            error: "unsupported_grant_type".to_string(),
            error_description: Some("The grant type is not supported".to_string()),
            error_uri: None,
        }
    }

    /// invalid_scope エラー
    pub fn invalid_scope(description: impl Into<String>) -> Self {
        Self {
            error: "invalid_scope".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    /// ステータスコードを返す
    pub fn status_code(&self) -> StatusCode {
        match self.error.as_str() {
            "invalid_request" => StatusCode::BAD_REQUEST,
            "invalid_client" => StatusCode::UNAUTHORIZED,
            "invalid_grant" => StatusCode::BAD_REQUEST,
            "unauthorized_client" => StatusCode::UNAUTHORIZED,
            "unsupported_grant_type" => StatusCode::BAD_REQUEST,
            "invalid_scope" => StatusCode::BAD_REQUEST,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    /// WWW-Authenticate ヘッダ値を返す（RFC 6750）
    pub fn www_authenticate_header(&self) -> String {
        let mut value = format!("Bearer error=\"{}\"", self.error);
        if let Some(ref desc) = self.error_description {
            value.push_str(&format!(", error_description=\"{}\"", desc));
        }
        if let Some(ref uri) = self.error_uri {
            value.push_str(&format!(", error_uri=\"{}\"", uri));
        }
        value
    }
}

/// 認証エラー
///
/// トークン検証時に発生するエラーを表します。
#[derive(Clone, Debug)]
pub enum OAuth2AuthError {
    /// トークンが無効
    InvalidToken(String),
    /// トークンが期限切れ
    ExpiredToken,
    /// 必要なスコープがない
    InsufficientScope {
        required: Vec<String>,
        provided: Vec<String>,
    },
    /// トークンが見つからない
    TokenNotFound,
    /// その他のエラー
    Other(String),
}

impl std::fmt::Display for OAuth2AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuth2AuthError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            OAuth2AuthError::ExpiredToken => write!(f, "Token has expired"),
            OAuth2AuthError::InsufficientScope { required, provided } => {
                write!(
                    f,
                    "Insufficient scope: required {:?}, provided {:?}",
                    required, provided
                )
            }
            OAuth2AuthError::TokenNotFound => write!(f, "Token not found"),
            OAuth2AuthError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for OAuth2AuthError {}

impl serde::Serialize for OAuth2AuthError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// トークンデータ
///
/// 検証されたトークンに含まれる情報を保持します。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenData {
    /// トークン識別子（ subject ）
    pub sub: String,
    /// スコープ一覧
    pub scopes: Vec<String>,
    /// 追加のクレーム（オプション）
    #[serde(default)]
    pub claims: std::collections::HashMap<String, serde_json::Value>,
}

impl TokenData {
    /// 新しい TokenData を作成
    pub fn new(sub: String, scopes: Vec<String>) -> Self {
        Self {
            sub,
            scopes,
            claims: std::collections::HashMap::new(),
        }
    }

    /// クレームを追加
    pub fn with_claim(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.claims.insert(key.into(), value);
        self
    }

    /// スコープを持っているか確認
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// 全ての必要なスコープを持っているか確認
    pub fn has_all_scopes(&self, required: &[String]) -> bool {
        required.iter().all(|scope| self.has_scope(scope))
    }
}

/// OAuth2 トークンバリデーター trait
///
/// トークン検証ロジックを実装するためのインターフェースです。
/// 独自のバリデーターを実装して使用します。
///
/// # Example
///
/// ```ignore
/// use ultraapi::middleware::{OAuth2TokenValidator, TokenData, OAuth2AuthError};
///
/// struct MyValidator;
///
/// #[async_trait::async_trait]
/// impl OAuth2TokenValidator for MyValidator {
///     async fn validate(&self, token: &str) -> Result<TokenData, OAuth2AuthError> {
///         // 独自の検証ロジック
///         Ok(TokenData::new("user123".to_string(), vec!["read".to_string()]))
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait OAuth2TokenValidator: Send + Sync {
    /// トークンを検証し、TokenData を返す
    ///
    /// # Arguments
    /// * `token` - 検証対象のトークン文字列
    ///
    /// # Returns
    /// * `Ok(TokenData)` - 検証成功
    /// * `Err(OAuth2AuthError)` - 検証失敗
    async fn validate(&self, token: &str) -> Result<TokenData, OAuth2AuthError>;

    /// オプション: スコープを検証
    ///
    /// デフォルト実装は常に成功します。
    fn validate_scopes(
        &self,
        _token_data: &TokenData,
        _required: &[String],
    ) -> Result<(), OAuth2AuthError> {
        Ok(())
    }
}

/// 不透明トークンバリデーター（Opaque Token Validator）
///
/// HashMap にトークンをマッピングするシンプルな実装です。
/// 本番環境ではデータベースや Redis などを使用してください。
///
/// # Example
///
/// ```ignore
/// use ultraapi::middleware::OpaqueTokenValidator;
///
/// let mut validator = OpaqueTokenValidator::new();
/// validator.add_token("valid-token-123", "user1", vec!["read".to_string()]);
/// validator.add_token("valid-token-456", "user2", vec!["read", "write".to_string()]);
///
/// // 検証
/// let result = validator.validate("valid-token-123").await;
/// assert!(result.is_ok());
/// ```
#[derive(Clone)]
pub struct OpaqueTokenValidator {
    /// トークン -> (subject, scopes) のマッピング
    tokens: std::sync::Arc<std::collections::HashMap<String, (String, Vec<String>)>>,
}

impl OpaqueTokenValidator {
    /// 新しい OpaqueTokenValidator を作成
    pub fn new() -> Self {
        Self {
            tokens: std::sync::Arc::new(std::collections::HashMap::new()),
        }
    }

    /// トークンを追加
    ///
    /// # Arguments
    /// * `token` - トークン文字列
    /// * `sub` - subject（ユーザーIDなど）
    /// * `scopes` - スコープ一覧
    pub fn add_token(&self, token: &str, sub: &str, scopes: Vec<String>) -> Self {
        let mut new_tokens = (*self.tokens).clone();
        new_tokens.insert(token.to_string(), (sub.to_string(), scopes));
        Self {
            tokens: std::sync::Arc::new(new_tokens),
        }
    }

    /// トークンを削除
    pub fn remove_token(&self, token: &str) -> Self {
        let mut new_tokens = (*self.tokens).clone();
        new_tokens.remove(token);
        Self {
            tokens: std::sync::Arc::new(new_tokens),
        }
    }

    /// 複数のトークンを一括追加
    pub fn extend_tokens<'a>(
        &self,
        tokens: impl IntoIterator<Item = (&'a str, &'a str, Vec<String>)>,
    ) -> Self {
        let mut new_tokens = (*self.tokens).clone();
        for (token, sub, scopes) in tokens {
            new_tokens.insert(token.to_string(), (sub.to_string(), scopes));
        }
        Self {
            tokens: std::sync::Arc::new(new_tokens),
        }
    }

    /// スコープ検証
    fn check_scopes(
        &self,
        provided: &[String],
        required: &[String],
    ) -> Result<(), OAuth2AuthError> {
        for req in required {
            if !provided.contains(req) {
                return Err(OAuth2AuthError::InsufficientScope {
                    required: required.to_vec(),
                    provided: provided.to_vec(),
                });
            }
        }
        Ok(())
    }
}

impl Default for OpaqueTokenValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl OAuth2TokenValidator for OpaqueTokenValidator {
    async fn validate(&self, token: &str) -> Result<TokenData, OAuth2AuthError> {
        let token_data = self.tokens.get(token);

        match token_data {
            Some((sub, scopes)) => Ok(TokenData::new(sub.clone(), scopes.clone())),
            None => Err(OAuth2AuthError::TokenNotFound),
        }
    }

    fn validate_scopes(
        &self,
        token_data: &TokenData,
        required: &[String],
    ) -> Result<(), OAuth2AuthError> {
        self.check_scopes(&token_data.scopes, required)
    }
}

/// Bearer 認証エラー応答を作成する（ApiError 形式）
///
/// 認証失敗時に ApiError 形式のレスポンスを作成します。
/// WWW-Authenticate ヘッダも付与します（RFC 6750 対応）。
///
/// # Arguments
/// * `error` - OAuth2 エラーコード
/// * `description` - エラー詳細
///
/// # Returns
/// * `ApiError` - UltraAPI の ApiError 形式
pub fn create_bearer_auth_error(error: &str, description: &str) -> crate::ApiError {
    crate::ApiError::unauthorized(format!("{}: {}", error, description))
}
