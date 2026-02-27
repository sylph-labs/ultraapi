// Middleware module for auth enforcement and middleware builder
use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Specifies where to look for credentials
#[derive(Clone, Debug, Default)]
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
    fn validate_scopes(&self, _credentials: &Credentials, _required_scopes: &[String]) -> Result<(), AuthError> {
        Ok(())
    }
}

/// Credentials extracted from the request
#[derive(Clone, Debug)]
pub struct Credentials {
    /// The authorization scheme (e.g., "bearer", "apiKey")
    pub scheme: String,
    /// The credential value (e.g., token, API key)
    pub value: String,
    /// The security scheme name used (e.g., "bearerAuth", "apiKeyAuth")
    pub security_scheme: Option<String>,
    /// The scopes that were validated (if any)
    pub scopes: Vec<String>,
}

impl Credentials {
    /// Create new credentials
    pub fn new(scheme: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            scheme: scheme.into(),
            value: value.into(),
            security_scheme: None,
            scopes: vec![],
        }
    }

    /// Create credentials with security scheme name
    pub fn with_scheme(scheme: impl Into<String>, value: impl Into<String>, security_scheme: impl Into<String>) -> Self {
        Self {
            scheme: scheme.into(),
            value: value.into(),
            security_scheme: Some(security_scheme.into()),
            scopes: vec![],
        }
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

    fn validate_scopes(&self, credentials: &Credentials, required_scopes: &[String]) -> Result<(), AuthError> {
        // First validate the credentials
        self.inner.validate(credentials)?;

        // If no scopes required, allow
        if required_scopes.is_empty() {
            return Ok(());
        }

        // Get scopes for this credential
        let granted_scopes = self.scope_map.get(&credentials.value)
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
                                    Credentials::with_scheme(scheme_name, scheme_value, &scheme.name)
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
                                error.into_response()
                            }
                        }
                    }
                    Err(auth_error) => {
                        let error = if auth_error.status == StatusCode::FORBIDDEN {
                            super::ApiError::forbidden(auth_error.message)
                        } else {
                            super::ApiError::unauthorized(auth_error.message)
                        };
                        error.into_response()
                    }
                }
            }
            None => {
                // No credentials found - check if any security scheme is configured
                // If security schemes are configured, require authentication
                if self.security_schemes.is_empty() {
                    // No schemes configured, try legacy behavior
                    let error = super::ApiError::unauthorized("Missing authorization header");
                    error.into_response()
                } else {
                    // Schemes configured but no credentials provided
                    let error = super::ApiError::unauthorized(
                        "Missing authentication credentials",
                    );
                    error.into_response()
                }
            }
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

/// Middleware builder for UltraAPI applications
pub struct MiddlewareBuilder {
    pub auth_enabled: bool,
    pub auth_layer: Option<AuthLayer>,
    pub cors_config: Option<CorsConfig>,
    pub compression_config: Option<CompressionConfig>,
    pub rate_limit_config: Option<RateLimitConfig>,
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
            rate_limit_config: None,
        }
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

    /// Add security scheme configurations to the auth layer
    /// This enables extraction of credentials from different locations (header, query, cookie)
    pub fn with_security_schemes(mut self, schemes: Vec<SecuritySchemeConfig>) -> Self {
        if let Some(auth_layer) = self.auth_layer.take() {
            self.auth_layer = Some(auth_layer.with_security_schemes(schemes));
        } else {
            // Create new auth layer with schemes
            self.auth_enabled = true;
            self.auth_layer = Some(AuthLayer::new(MockAuthValidator::new()).with_security_schemes(schemes));
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
}

// ============================================================================
// Rate Limiting Middleware
// ============================================================================

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// レート制限の設定
/// 
/// # Example
/// 
/// ```rust
/// use ultraapi::prelude::*;
/// 
/// let app = UltraApiApp::new()
///     .rate_limit(RateLimitConfig::new(10, Duration::from_secs(60)));
/// ```
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed within the time window
    pub max_requests: u32,
    /// Time window duration
    pub window: Duration,
    /// Custom key function - if None, uses default (X-Forwarded-For or "global")
    pub key_fn: Option<Arc<dyn Fn(&axum::extract::Request) -> String + Send + Sync>>,
}

impl RateLimitConfig {
    /// Create a new rate limit config
    /// 
    /// # Arguments
    /// 
    /// * `max_requests` - Maximum number of requests allowed
    /// * `window` - Time window for the rate limit
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use ultraapi::prelude::*;
    /// use std::time::Duration;
    /// 
    /// // Allow 10 requests per minute
    /// let config = RateLimitConfig::new(10, Duration::from_secs(60));
    /// ```
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            key_fn: None,
        }
    }

    /// Set a custom key function for rate limiting
    /// 
    /// The key is used to track requests per client. By default, uses X-Forwarded-For header
    /// (first IP) if available, otherwise uses "global".
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use ultraapi::prelude::*;
    /// 
    /// let config = RateLimitConfig::new(10, Duration::from_secs(60))
    ///     .with_key_fn(|req| {
    ///         // Use Authorization header as key
    ///         req.headers()
    ///             .get("authorization")
    ///             .map(|h| h.to_str().unwrap_or("unknown").to_string())
    ///             .unwrap_or_else(|| "anonymous".to_string())
    ///     });
    /// ```
    pub fn with_key_fn<F>(mut self, key_fn: F) -> Self
    where
        F: Fn(&axum::extract::Request) -> String + Send + Sync + 'static,
    {
        self.key_fn = Some(Arc::new(key_fn));
        self
    }

    /// Build the rate limit layer
    pub fn build(self) -> RateLimitLayer {
        RateLimitLayer {
            config: Arc::new(self),
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Rate limit key with request count and window start time
#[derive(Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

/// Tower middleware layer for rate limiting
#[derive(Clone)]
pub struct RateLimitLayer {
    config: Arc<RateLimitConfig>,
    store: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl<S, B> tower::Service<axum::http::Request<B>> for RateLimitLayer
where
    S: tower::Service<axum::http::Request<B>>,
    B: axum::body::HttpBody,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        // We don't hold any state, just delegate
        // This is a simple middleware that doesn't need to hold state
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::http::Request<B>) -> Self::Future {
        let config = self.config.clone();
        let store = self.store.clone();

        async move {
            // Get the rate limit key
            let key = if let Some(ref key_fn) = config.key_fn {
                key_fn(&req)
            } else {
                // Default key: X-Forwarded-For (first IP) or "global"
                req.headers()
                    .get("x-forwarded-for")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.split(',').next())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "global".to_string())
            };

            // Check rate limit
            let now = Instant::now();
            let mut store_guard = store.write();
            
            // Get or create entry
            let entry = store_guard.entry(key.clone()).or_insert_with(|| RateLimitEntry {
                count: 0,
                window_start: now,
            });

            // Check if we're in the same window
            let elapsed = now.duration_since(entry.window_start);
            if elapsed >= config.window {
                // Reset window
                entry.count = 1;
                entry.window_start = now;
            } else {
                entry.count += 1;
            }

            let remaining = config.max_requests.saturating_sub(entry.count);
            let retry_after = if entry.count > config.max_requests {
                // Calculate retry-after duration
                let wait_time = config.window - elapsed;
                Some(wait_time.as_secs())
            } else {
                None
            };

            // Release lock before proceeding
            drop(store_guard);

            // If rate limited, return 429
            if entry.count > config.max_requests {
                let error_body = serde_json::json!({
                    "error": "Too Many Requests",
                    "details": ["Rate limit exceeded. Please try again later."]
                });

                let mut response = axum::http::Response::builder()
                    .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
                    .header("content-type", "application/json")
                    .header("x-ratelimit-limit", config.max_requests.to_string())
                    .header("x-ratelimit-remaining", "0");

                if let Some(retry) = retry_after {
                    response = response.header("retry-after", retry.to_string());
                }

                return Ok(response
                    .body(axum::body::Body::from(error_body.to_string()))
                    .unwrap());
            }

            // Add rate limit headers to the request for the inner service
            let (mut parts, body) = req.into_parts();
            parts.headers.insert(
                "x-ratelimit-limit".parse().unwrap(),
                config.max_requests.to_string().parse().unwrap(),
            );
            parts.headers.insert(
                "x-ratelimit-remaining".parse().unwrap(),
                remaining.to_string().parse().unwrap(),
            );

            Ok(axum::http::Response::from_parts(parts, body))
        }
    }
}

/// Transform our service into a Layer for use with axum
impl<S> axum::middleware::Layer<S> for RateLimitLayer {
    fn layer(&self, inner: S) -> Self::Service {
        // We need to adapt our Service to work with axum's middleware
        // This is done through a custom wrapper
        RateLimitService {
            inner,
            layer: self.clone(),
        }
    }
}

/// Wrapper service that combines the rate limit layer with the inner service
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    layer: RateLimitLayer,
}

impl<S, B> tower::Service<axum::http::Request<B>> for RateLimitService<S>
where
    S: tower::Service<axum::http::Request<B>>,
    B: axum::body::HttpBody,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<B>) -> Self::Future {
        let config = self.layer.config.clone();
        let store = self.layer.store.clone();

        let key = if let Some(ref key_fn) = config.key_fn {
            key_fn(&req)
        } else {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "global".to_string())
        };

        // Check rate limit synchronously (since we use parking_lot)
        let now = Instant::now();
        let mut store_guard = store.write();
        
        let entry = store_guard.entry(key.clone()).or_insert_with(|| RateLimitEntry {
            count: 0,
            window_start: now,
        });

        let elapsed = now.duration_since(entry.window_start);
        if elapsed >= config.window {
            entry.count = 1;
            entry.window_start = now;
        } else {
            entry.count += 1;
        }

        let remaining = config.max_requests.saturating_sub(entry.count);
        let retry_after = if entry.count > config.max_requests {
            Some((config.window - elapsed).as_secs())
        } else {
            None
        };

        drop(store_guard);

        if entry.count > config.max_requests {
            let error_body = serde_json::json!({
                "error": "Too Many Requests",
                "details": ["Rate limit exceeded. Please try again later."]
            });

            let mut response = axum::http::Response::builder()
                .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
                .header("content-type", "application/json")
                .header("x-ratelimit-limit", config.max_requests.to_string())
                .header("x-ratelimit-remaining", "0");

            if let Some(retry) = retry_after {
                response = response.header("retry-after", retry.to_string());
            }

            let res = response
                .body(axum::body::Body::from(error_body.to_string()))
                .unwrap();
            
            // Return a boxed future that resolves immediately
            return Box::pin(async { Ok(res) });
        }

        // Add headers and continue
        let (mut parts, body) = req.into_parts();
        parts.headers.insert(
            "x-ratelimit-limit".parse().unwrap(),
            config.max_requests.to_string().parse().unwrap(),
        );
        parts.headers.insert(
            "x-ratelimit-remaining".parse().unwrap(),
            remaining.to_string().parse().unwrap(),
        );

        let req = axum::http::Request::from_parts(parts, body);
        self.inner.call(req)
    }
}
