// Response Cookies Tests
// Tests for CookieResponse to add Set-Cookie headers to responses

use ultraapi::prelude::*;

// Test models
#[api_model]
#[derive(Debug, Clone)]
struct Message {
    pub status: String,
}

// --- Test 1: Single cookie with basic options ---

/// Single cookie endpoint
#[get("/cookie/single")]
#[response_class("cookie")]
async fn single_cookie() -> CookieResponse<Message> {
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie("session", "abc123")
}

// --- Test 2: Multiple cookies ---

/// Multiple cookies endpoint
#[get("/cookie/multiple")]
#[response_class("cookie")]
async fn multiple_cookies() -> CookieResponse<Message> {
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie("session", "abc123")
        .cookie("user_id", "42")
        .cookie("theme", "dark")
}

// --- Test 3: Cookie with HttpOnly ---

/// Cookie with HttpOnly flag
#[get("/cookie/http-only")]
#[response_class("cookie")]
async fn cookie_http_only() -> CookieResponse<Message> {
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie_options("session", "abc123", |opts| opts.http_only())
}

// --- Test 4: Cookie with Secure ---

/// Cookie with Secure flag
#[get("/cookie/secure")]
#[response_class("cookie")]
async fn cookie_secure() -> CookieResponse<Message> {
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie_options("session", "abc123", |opts| opts.secure())
}

// --- Test 5: Cookie with SameSite ---

/// Cookie with SameSite=Lax
#[get("/cookie/same-site-lax")]
#[response_class("cookie")]
async fn cookie_same_site_lax() -> CookieResponse<Message> {
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie_options("session", "abc123", |opts| opts.same_site_lax())
}

// --- Test 6: Cookie with Path ---

/// Cookie with Path
#[get("/cookie/path")]
#[response_class("cookie")]
async fn cookie_path() -> CookieResponse<Message> {
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie_options("session", "abc123", |opts| opts.path("/api"))
}

// --- Test 7: Cookie with Max-Age ---

/// Cookie with Max-Age
#[get("/cookie/max-age")]
#[response_class("cookie")]
async fn cookie_max_age() -> CookieResponse<Message> {
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie_options("session", "abc123", |opts| opts.max_age(3600))
}

// --- Test 8: Cookie with Expires ---

/// Cookie with Expires
#[get("/cookie/expires")]
#[response_class("cookie")]
async fn cookie_expires() -> CookieResponse<Message> {
    use time::OffsetDateTime;
    
    // Set expiration to 1 day from now
    let expires = OffsetDateTime::now_utc() + time::Duration::days(1);
    
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie_options("session", "abc123", |opts| opts.expires(expires))
}

// --- Test 9: Multiple cookies with mixed options ---

/// Multiple cookies with different options
#[get("/cookie/mixed")]
#[response_class("cookie")]
async fn cookie_mixed() -> CookieResponse<Message> {
    use time::OffsetDateTime;
    
    let expires = OffsetDateTime::now_utc() + time::Duration::days(7);
    
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie("tracking", "xyz789")
        .cookie_options("session", "abc123", |opts| opts.http_only().secure().path("/"))
        .cookie_options("preference", "dark", |opts| opts.same_site_lax().max_age(86400))
        .cookie_options("persistent", "value1", |opts| opts.expires(expires))
}

// --- Test 10: Cookie with all options combined ---

/// Cookie with all options
#[get("/cookie/all-options")]
#[response_class("cookie")]
async fn cookie_all_options() -> CookieResponse<Message> {
    use time::OffsetDateTime;
    
    let expires = OffsetDateTime::now_utc() + time::Duration::days(30);
    
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie_options("session", "full-options-cookie", |opts| {
            opts.http_only()
                .secure()
                .same_site_strict()
                .path("/api")
                .max_age(86400)
                .expires(expires)
        })
}

// --- Test 11: SameSite None (requires Secure) ---

/// Cookie with SameSite=None (requires Secure)
#[get("/cookie/same-site-none")]
#[response_class("cookie")]
async fn cookie_same_site_none() -> CookieResponse<Message> {
    CookieResponse::new(Message { status: "ok".to_string() })
        .cookie_options("cross_site", "value", |opts| opts.same_site_none().secure())
}

// --- Helper to spawn app ---

async fn spawn_app() -> String {
    let app = UltraApiApp::new()
        .title("Response Cookies Test API")
        .version("0.1.0")
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        ultraapi::axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

// --- Tests ---

#[tokio::test]
async fn test_single_cookie() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/single", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    // Check for Set-Cookie header
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain the session cookie
    assert!(cookie_str.contains("session=abc123"));
}

#[tokio::test]
async fn test_multiple_cookies() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/multiple", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    // Get all Set-Cookie headers (may be multiple or comma-separated)
    let set_cookie_headers = response.headers().get_all("set-cookie");
    let cookies: Vec<String> = set_cookie_headers
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();
    
    eprintln!("Cookies: {:?}", cookies);
    
    // Combine all cookies into a single string for checking
    let all_cookies = cookies.join("; ");
    eprintln!("All cookies combined: {}", all_cookies);
    
    // Should contain all 3 cookies (either as separate headers or comma-separated)
    assert!(all_cookies.contains("session=abc123"), "Missing session cookie: {}", all_cookies);
    assert!(all_cookies.contains("user_id=42"), "Missing user_id cookie: {}", all_cookies);
    assert!(all_cookies.contains("theme=dark"), "Missing theme cookie: {}", all_cookies);
}

#[tokio::test]
async fn test_cookie_http_only() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/http-only", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain HttpOnly
    assert!(cookie_str.contains("HttpOnly"));
    assert!(cookie_str.contains("session=abc123"));
}

#[tokio::test]
async fn test_cookie_secure() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/secure", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain Secure
    assert!(cookie_str.contains("Secure"));
    assert!(cookie_str.contains("session=abc123"));
}

#[tokio::test]
async fn test_cookie_same_site_lax() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/same-site-lax", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain SameSite=Lax
    assert!(cookie_str.contains("SameSite=Lax"));
    assert!(cookie_str.contains("session=abc123"));
}

#[tokio::test]
async fn test_cookie_path() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/path", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain Path
    assert!(cookie_str.contains("Path=/api"));
    assert!(cookie_str.contains("session=abc123"));
}

#[tokio::test]
async fn test_cookie_max_age() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/max-age", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain Max-Age
    assert!(cookie_str.contains("Max-Age=3600"));
    assert!(cookie_str.contains("session=abc123"));
}

#[tokio::test]
async fn test_cookie_expires() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/expires", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain Expires
    assert!(cookie_str.contains("Expires="));
    assert!(cookie_str.contains("session=abc123"));
}

#[tokio::test]
async fn test_cookie_mixed_options() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/mixed", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    // Get all Set-Cookie headers
    let set_cookie_headers = response.headers().get_all("set-cookie");
    let cookies: Vec<String> = set_cookie_headers
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();
    
    // Combine all cookies for checking
    let all_cookies = cookies.join("; ");
    
    // First cookie (no options)
    assert!(all_cookies.contains("tracking=xyz789"), "Missing tracking cookie");
    
    // Second cookie (http_only, secure, path)
    assert!(all_cookies.contains("session=abc123"), "Missing session cookie");
    assert!(all_cookies.contains("HttpOnly"), "Missing HttpOnly");
    assert!(all_cookies.contains("Secure"), "Missing Secure");
    assert!(all_cookies.contains("Path=/"), "Missing Path=/");
    
    // Third cookie (same_site_lax, max_age)
    assert!(all_cookies.contains("preference=dark"), "Missing preference cookie");
    assert!(all_cookies.contains("SameSite=Lax"), "Missing SameSite=Lax");
    assert!(all_cookies.contains("Max-Age=86400"), "Missing Max-Age=86400");
    
    // Fourth cookie (expires)
    assert!(all_cookies.contains("persistent=value1"), "Missing persistent cookie");
    assert!(all_cookies.contains("Expires="), "Missing Expires");
}

#[tokio::test]
async fn test_cookie_all_options() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/all-options", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain all options
    assert!(cookie_str.contains("session=full-options-cookie"));
    assert!(cookie_str.contains("HttpOnly"));
    assert!(cookie_str.contains("Secure"));
    assert!(cookie_str.contains("SameSite=Strict"));
    assert!(cookie_str.contains("Path=/api"));
    assert!(cookie_str.contains("Max-Age=86400"));
    assert!(cookie_str.contains("Expires="));
}

#[tokio::test]
async fn test_cookie_same_site_none() {
    let base_url = spawn_app().await;
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/cookie/same-site-none", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let set_cookie = response.headers().get("set-cookie").unwrap();
    let cookie_str = set_cookie.to_str().unwrap();
    
    // Should contain SameSite=None and Secure (required for SameSite=None)
    assert!(cookie_str.contains("SameSite=None"));
    assert!(cookie_str.contains("Secure"));
    assert!(cookie_str.contains("cross_site=value"));
}
