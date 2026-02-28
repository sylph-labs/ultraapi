//! Session cookies (server-side sessions)
//!
//! UltraAPI のサーバーサイドセッション（Cookie + in-memory store）です。
//!
//! - Cookie には session_id のみを保持
//! - サーバ側に session data を保存（HashMap）
//! - TTL により期限切れを自動無効化
//!
//! NOTE:
//! - `SessionConfig::secret` は将来的な署名/暗号化用に予約しています（MVP では未使用）。

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderMap, StatusCode},
    response::Response,
};
use axum_extra::extract::cookie::{Cookie, SameSite as CookieSameSite};
use base64::Engine;
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// SameSite cookie policy
#[derive(Clone, Copy, Debug, Default)]
pub enum SameSite {
    Strict,
    #[default]
    Lax,
    None,
}

impl SameSite {
    fn to_cookie(self) -> CookieSameSite {
        match self {
            SameSite::Strict => CookieSameSite::Strict,
            SameSite::Lax => CookieSameSite::Lax,
            SameSite::None => CookieSameSite::None,
        }
    }
}

/// Session configuration
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// Secret key for signing/encrypting session IDs (reserved; not used in MVP)
    pub secret: String,
    /// Session TTL (default: 24 hours)
    pub ttl: Duration,
    /// Cookie name (default: "session_id")
    pub cookie_name: String,
    /// Cookie path (default: "/")
    pub cookie_path: String,
    /// Cookie domain (optional)
    pub cookie_domain: Option<String>,
    /// HttpOnly flag (default: true)
    pub http_only: bool,
    /// Secure flag (default: false)
    pub secure: bool,
    /// SameSite policy (default: Lax)
    pub same_site: SameSite,
}

impl SessionConfig {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            ttl: Duration::from_secs(24 * 60 * 60),
            cookie_name: "session_id".to_string(),
            cookie_path: "/".to_string(),
            cookie_domain: None,
            http_only: true,
            secure: false,
            same_site: SameSite::Lax,
        }
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn cookie_name(mut self, cookie_name: impl Into<String>) -> Self {
        self.cookie_name = cookie_name.into();
        self
    }

    pub fn cookie_path(mut self, cookie_path: impl Into<String>) -> Self {
        self.cookie_path = cookie_path.into();
        self
    }

    pub fn cookie_domain(mut self, cookie_domain: impl Into<String>) -> Self {
        self.cookie_domain = Some(cookie_domain.into());
        self
    }

    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }
}

#[derive(Clone)]
struct SessionEntry {
    data: HashMap<String, serde_json::Value>,
    expires_at: Instant,
}

impl SessionEntry {
    fn new(ttl: Duration) -> Self {
        Self {
            data: HashMap::new(),
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    fn refresh(&mut self, ttl: Duration) {
        self.expires_at = Instant::now() + ttl;
    }
}

/// In-memory session store
#[derive(Clone)]
pub struct SessionStore {
    ttl: Duration,
    sessions: Arc<RwLock<HashMap<String, SessionEntry>>>,
}

impl SessionStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn get_entry(&self, session_id: &str) -> Option<SessionEntry> {
        let store = self.sessions.read();
        store.get(session_id).cloned()
    }

    fn insert_entry(&self, session_id: String, entry: SessionEntry) {
        let mut store = self.sessions.write();
        store.insert(session_id, entry);
    }

    fn remove_entry(&self, session_id: &str) {
        let mut store = self.sessions.write();
        store.remove(session_id);
    }

    fn refresh_if_present(&self, session_id: &str) -> bool {
        let mut store = self.sessions.write();
        if let Some(entry) = store.get_mut(session_id) {
            if entry.is_expired() {
                store.remove(session_id);
                return false;
            }
            entry.refresh(self.ttl);
            return true;
        }
        false
    }

    fn ensure_session(&self, existing: Option<String>) -> (String, bool) {
        if let Some(id) = existing {
            if self.refresh_if_present(&id) {
                return (id, false);
            }
        }

        let id = self.generate_session_id();
        self.insert_entry(id.clone(), SessionEntry::new(self.ttl));
        (id, true)
    }

    fn with_entry_mut<R>(
        &self,
        session_id: &str,
        f: impl FnOnce(&mut SessionEntry) -> R,
    ) -> Option<R> {
        let mut store = self.sessions.write();
        let entry = store.get_mut(session_id)?;
        if entry.is_expired() {
            store.remove(session_id);
            return None;
        }
        entry.refresh(self.ttl);
        Some(f(entry))
    }

    fn generate_session_id(&self) -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let ctr = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        let mut buf = [0u8; 24];
        buf[..16].copy_from_slice(&nanos.to_be_bytes());
        buf[16..].copy_from_slice(&ctr.to_be_bytes());

        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
    }
}

#[derive(Clone)]
struct SessionRequestState {
    id: String,
    is_new: bool,
    /// 0 = clean, 1 = modified, 2 = cleared
    action: Arc<AtomicU8>,
}

impl SessionRequestState {
    fn mark_modified(&self) {
        self.action.store(1, Ordering::SeqCst);
    }

    fn mark_cleared(&self) {
        self.action.store(2, Ordering::SeqCst);
    }
}

/// Session extractor
#[derive(Clone)]
pub struct Session {
    state: SessionRequestState,
    store: SessionStore,
}

impl Session {
    /// Session ID
    pub fn id(&self) -> &str {
        &self.state.id
    }

    /// Get a typed value from session
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let entry = self.store.get_entry(&self.state.id)?;
        if entry.is_expired() {
            return None;
        }
        entry
            .data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Insert a value into session
    pub fn insert<T: Serialize>(
        &self,
        key: impl Into<String>,
        value: T,
    ) -> Result<(), serde_json::Error> {
        let key = key.into();
        let value = serde_json::to_value(value)?;

        let _ = self.store.with_entry_mut(&self.state.id, |entry| {
            entry.data.insert(key, value);
        });
        self.state.mark_modified();
        Ok(())
    }

    /// Remove a key from session
    pub fn remove(&self, key: &str) {
        let _ = self.store.with_entry_mut(&self.state.id, |entry| {
            entry.data.remove(key);
        });
        self.state.mark_modified();
    }

    /// Clear entire session
    pub fn clear(&self) {
        self.store.remove_entry(&self.state.id);
        self.state.mark_cleared();
    }
}

impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let store = parts
            .extensions
            .get::<SessionStore>()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "SessionStore missing"))?
            .clone();

        let req_state = parts
            .extensions
            .get::<SessionRequestState>()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "SessionRequestState missing",
            ))?
            .clone();

        Ok(Session {
            state: req_state,
            store,
        })
    }
}

/// Session layer (middleware)
#[derive(Clone)]
pub struct SessionLayer {
    config: SessionConfig,
    store: SessionStore,
}

impl SessionLayer {
    pub fn new(config: SessionConfig) -> Self {
        let store = SessionStore::new(config.ttl);
        Self { config, store }
    }
}

impl<S> tower::Layer<S> for SessionLayer {
    type Service = SessionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SessionService {
            inner,
            config: self.config.clone(),
            store: self.store.clone(),
        }
    }
}

#[derive(Clone)]
pub struct SessionService<S> {
    inner: S,
    config: SessionConfig,
    store: SessionStore,
}

impl<S, B> tower::Service<axum::http::Request<B>> for SessionService<S>
where
    S: tower::Service<axum::http::Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: std::fmt::Debug,
    B: Send + 'static,
{
    type Response = Response;
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

    fn call(&mut self, mut req: axum::http::Request<B>) -> Self::Future {
        let config = self.config.clone();
        let store = self.store.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // parse cookie
            let existing = req
                .headers()
                .get(header::COOKIE)
                .and_then(|h| h.to_str().ok())
                .and_then(|cookie_str| {
                    cookie_str.split(';').map(|s| s.trim()).find_map(|c| {
                        let (k, v) = c.split_once('=')?;
                        if k == config.cookie_name {
                            Some(v.to_string())
                        } else {
                            None
                        }
                    })
                });

            let (session_id, is_new) = store.ensure_session(existing);
            let action = Arc::new(AtomicU8::new(0));

            let req_state = SessionRequestState {
                id: session_id.clone(),
                is_new,
                action: action.clone(),
            };

            req.extensions_mut().insert(store.clone());
            req.extensions_mut().insert(req_state.clone());

            let mut res = inner.call(req).await.unwrap();

            let action_val = action.load(Ordering::SeqCst);
            let should_set = action_val == 1;
            let should_clear = action_val == 2;

            if should_set || should_clear {
                let mut cookie = if should_clear {
                    Cookie::new(config.cookie_name.clone(), "")
                } else {
                    Cookie::new(config.cookie_name.clone(), session_id)
                };

                cookie.set_path(config.cookie_path.clone());
                cookie.set_http_only(config.http_only);
                cookie.set_secure(config.secure);
                cookie.set_same_site(config.same_site.to_cookie());

                if let Some(domain) = config.cookie_domain.clone() {
                    cookie.set_domain(domain);
                }

                // Max-Age
                if should_clear {
                    cookie.set_max_age(time::Duration::seconds(0));
                } else {
                    cookie.set_max_age(time::Duration::seconds(config.ttl.as_secs() as i64));
                }

                res.headers_mut()
                    .append(header::SET_COOKIE, cookie.to_string().parse().unwrap());
            }

            Ok(res)
        })
    }
}

/// Utility to read session cookie value from headers
pub fn extract_session_cookie(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .find_map(|set_cookie| {
            // take only "name=value" part
            let kv = set_cookie.split(';').next()?;
            let (k, v) = kv.split_once('=')?;
            if k.trim() == cookie_name {
                Some(v.trim().to_string())
            } else {
                None
            }
        })
}
