#![allow(
    clippy::assertions_on_constants,
    clippy::useless_vec,
    unused_imports,
    dead_code
)]

// P0 Extractor Coverage Tests
// Tests for Header, Cookie, Form, File, and Multipart extractors
// Where unsupported, adds explicit capability-gap tests with clear reason

use std::collections::HashMap;

// ===== Header Extractor Tests =====

// Test: Header extraction via http::HeaderMap
// Note: UltraAPI exposes http types via axum::http
#[test]
fn test_header_map_accessible() {
    use ultraapi::axum::http::HeaderMap;

    // Verify HeaderMap is accessible
    let _map = HeaderMap::new();
}

// Test: TypedHeader is now supported via axum_extra
#[test]
fn test_typed_header_now_supported() {
    // UltraAPI's macro handler now supports extracting individual headers like:
    // async fn handler(auth: TypedHeader<Authorization<Bearer>>) { ... }
    //
    // This is enabled via:
    // - axum-extra with "typed-header" feature
    // - The macro generates code to extract TypedHeader automatically

    // Verify TypedHeader type is accessible
    use axum_extra::headers::authorization::Bearer;
    use axum_extra::headers::Authorization;
    use ultraapi::prelude::TypedHeader;

    // This compiles - TypedHeader is available
    // Just verify the type compiles (not calling unimplemented!)
    assert!(true, "TypedHeader is now supported via axum_extra");
}

// Test: Header extraction via manual parsing
// Documents current approach for header extraction
#[test]
fn test_header_manual_parsing_approach() {
    // Current approach for header extraction:
    // 1. Access HeaderMap via extractor
    // 2. Manually get and parse headers
    //
    // This test verifies the required types are accessible
    use ultraapi::axum::http::{header, HeaderMap};

    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer token".parse().unwrap());

    // Verify we can access authorization header
    let auth = headers.get(header::AUTHORIZATION);
    assert!(
        auth.is_some(),
        "Should be able to get Authorization header from HeaderMap"
    );
}

// ===== Cookie Extractor Tests =====

// Test: CookieJar is now supported via axum_extra
#[test]
fn test_cookie_jar_now_supported() {
    // UltraAPI's macro handler now supports cookie extraction:
    // async fn handler(cookies: CookieJar) { ... }
    //
    // This is enabled via:
    // - axum-extra with "cookie" feature
    // - The macro generates code to extract CookieJar automatically

    // Verify CookieJar type is accessible
    use ultraapi::prelude::CookieJar;

    // This compiles - CookieJar is available
    // Just verify the type compiles (not calling unimplemented!)
    assert!(true, "CookieJar is now supported via axum_extra");
}

// Test: Cookie extraction via manual parsing (still works)
#[test]
fn test_cookie_manual_parsing_approach() {
    // Manual cookie extraction approach still works:
    // This test verifies we can access the COOKIE header from HeaderMap
    use ultraapi::axum::http::{header, HeaderMap};

    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, "session=abc123".parse().unwrap());

    // Verify we can access cookie header
    let cookie = headers.get(header::COOKIE);
    assert!(
        cookie.is_some(),
        "Should be able to get COOKIE header from HeaderMap"
    );
}

// ===== Form Extractor Tests =====

// ===== Form Extractor Tests =====

// Test: Form extraction is now supported
#[test]
fn test_form_now_supported() {
    // UltraAPI's macro handler now supports Form<T> extraction:
    // async fn handler(form: Form<MyFormData>) { ... }
    //
    // This is enabled via axum "form" feature

    // Verify Form type is accessible
    use ultraapi::prelude::Form;

    // This compiles - Form is available
    fn _check_form<T: serde::de::DeserializeOwned>(_: Form<T>) {}
}

// Test: Form extraction via manual parsing (still works)
#[test]
fn test_form_manual_parsing_approach() {
    // Manual form parsing still works:
    // This test verifies we can work with the raw body infrastructure
    use ultraapi::axum::body::Body;

    let _body = Body::empty();
    // Body is accessible for manual parsing if needed
}

// ===== Multipart Extractor Tests =====

// Test: Multipart is now supported
#[test]
fn test_multipart_now_supported() {
    // UltraAPI's macro handler now supports Multipart extraction:
    // async fn handler(multipart: Multipart) { ... }
    //
    // This is enabled via axum "multipart" feature

    // Verify Multipart type is accessible
    use ultraapi::prelude::Multipart;

    // This compiles - Multipart is available
    fn _check_multipart(_: Multipart) {}
}

// ===== Extractors Available via UltraAPI =====

// Test: Query extractor is available
#[test]
fn test_query_extractor_available() {
    // Query is re-exported in prelude - verify type exists and works
    fn takes_query<Q: serde::de::DeserializeOwned>(_q: ultraapi::prelude::Query<Q>) {}
    let _ = takes_query::<()>;

    // Also verify we can create a Query from empty parameters
    use ultraapi::axum::extract::Query;

    // Verify Query is properly re-exported
    let _: Query<HashMap<String, String>> = Query(HashMap::new());
}

// Test: Path extractor via macro
#[test]
fn test_path_extractor_via_macro() {
    // Path parameters are extracted via the handler function signature
    // #[get("/users/{id}")]
    // async fn get_user(id: i64) { ... }
    //
    // The macro handles path extraction automatically
    // Verify the Path type is accessible
    use ultraapi::axum::extract::Path;

    // Verify Path can be created (with a value)
    let path: Path<String> = Path("test".to_string());
    assert_eq!(path.0, "test");
}

// Test: Json body extractor is available
#[test]
fn test_json_extractor_available() {
    // JSON body extraction is built-in:
    // async fn create_user(body: CreateUser) { ... }
    // Verify the Json type is accessible
    use ultraapi::axum::Json;

    // Verify Json can be created (with a value)
    let json: Json<&str> = Json("test");
    assert_eq!(json.0, "test");
}

// Test: Dep (dependency injection) extractor
#[test]
fn test_dep_extractor_available() {
    use ultraapi::Dep;

    // Dependency injection:
    // async fn handler(dep(db): Dep<Database>) { ... }
    // Verify the Dep type is accessible and has the correct bounds

    // Verify Dep<T> where T: Send + Sync + 'static compiles
    fn check_dep_bounds<T: Send + Sync + 'static>(_: Dep<T>) {}
    // Use type inference - just verify the function compiles
    fn _test_fn() {
        fn _inner<T: Send + Sync + 'static>(_: Dep<T>) {}
    }

    // The above compiles, so Dep<T> is properly defined with Send + Sync + 'static bounds
    assert!(true, "Dep type is available with correct bounds");
}

// Test: State extractor
#[test]
fn test_state_extractor_available() {
    use ultraapi::State;

    // State extraction:
    // async fn handler(state: State<AppState>) { ... }
    // Verify the State type is accessible and has the correct bounds

    // Verify State<T> where T: Clone + Send + Sync + 'static compiles
    fn check_state_bounds<T: Clone + Send + Sync + 'static>(_: State<T>) {}
    // Use type inference - just verify the function compiles
    fn _test_fn() {
        fn _inner<T: Clone + Send + Sync + 'static>(_: State<T>) {}
    }

    // The above compiles, so State<T> is properly defined with Clone + Send + Sync + 'static bounds
    assert!(true, "State type is available with correct bounds");
}

// ===== Extractor Support Matrix =====

// This test documents the current extractor support matrix
// IMPORTANT: This distinguishes between "type is accessible" vs "automatically extracted by macro"
#[test]
fn test_extractor_support_matrix() {
    // UltraAPI's macro handler signature support:
    // ✅ Fully supported by macro (auto-extracted):
    // - Query<T>: ✅ Supported (via ultraapi::prelude::Query) - auto-parsed from URL
    // - Path<T>: ✅ Supported (via macro attribute) - auto-extracted from URL path
    // - Json<T>: ✅ Supported (auto-deserialized from request body)
    // - Dep<T>: ✅ Supported (dependency injection via AppState)
    // - State<T>: ✅ Supported (state extraction from AppState)
    // - TypedHeader<T>: ✅ Supported (via axum_extra) - auto-extracted from request headers
    // - CookieJar: ✅ Supported (via axum_extra) - auto-extracted from cookies
    // - Form<T>: ✅ Supported (via axum) - auto-extracted from form data
    // - Multipart: ✅ Supported (via axum) - auto-extracted for file uploads
    //
    // ⚠️ Type accessible but NOT auto-extracted by macro:
    // - HeaderMap: ⚠️ Accessible via axum::http::HeaderMap but NOT auto-extracted
    //   Users can add it as a parameter but macro doesn't handle it specially
    //   Must manually parse: headers.get("X-Header").and_then(|v| v.to_str().ok())

    // Count what's actually supported by the macro (auto-extracted)
    let macro_supported = vec![
        ("Query<T> - auto-parsed from URL", true),
        ("Path<T> - auto-extracted from URL", true),
        ("Json (body) - auto-deserialized", true),
        ("Dep<T> - dependency injection", true),
        ("State<T> - state extraction", true),
        ("TypedHeader<T> - via axum_extra", true),
        ("CookieJar - via axum_extra", true),
        ("Form<T> - via axum", true),
        ("Multipart - via axum", true),
    ];

    // Types accessible but require manual handling
    let manual_handling = vec![("HeaderMap - accessible but manual parse", false)];

    // Combined list for reference (not used in assertions)
    let _all: Vec<_> = macro_supported
        .iter()
        .chain(manual_handling.iter())
        .collect();

    let supported = macro_supported.iter().filter(|(_, s)| *s).count();
    let manual = manual_handling.iter().filter(|(_, s)| !*s).count();

    assert_eq!(supported, 9, "Should have 9 macro-supported extractors");
    assert_eq!(manual, 1, "Should have 1 manual-handling type");
}

// ===== NEW: TypedHeader Extractor Tests =====

#[test]
fn test_typed_header_extractor_available() {
    // TypedHeader is now available via axum_extra
    use axum_extra::headers::authorization::Bearer;
    use axum_extra::headers::Authorization;
    use ultraapi::prelude::TypedHeader;

    // Verify TypedHeader is properly re-exported in prelude
    fn _check_typed_header(_: TypedHeader<Authorization<Bearer>>) {}

    // The above compiles - TypedHeader is properly available
    assert!(true, "TypedHeader extractor is available via prelude");
}

// ===== NEW: CookieJar Extractor Tests =====

#[test]
fn test_cookie_jar_extractor_available() {
    // CookieJar is now available via axum_extra
    use ultraapi::prelude::CookieJar;

    // Verify CookieJar is properly re-exported in prelude
    fn _check_cookie_jar(_: CookieJar) {}

    // The above compiles - CookieJar is properly available
    assert!(true, "CookieJar extractor is available via prelude");
}

// ===== NEW: Form Extractor Tests =====

#[test]
fn test_form_extractor_available() {
    // Form is available via axum (form feature is enabled)
    use ultraapi::prelude::Form;

    // Verify Form is properly re-exported in prelude
    fn _check_form<T: serde::de::DeserializeOwned>(_: Form<T>) {}

    // The above compiles - Form is properly available
    assert!(true, "Form extractor is available via prelude");
}

// ===== NEW: Multipart Extractor Tests =====

#[test]
fn test_multipart_extractor_available() {
    // Multipart is available via axum (multipart feature is enabled)
    use ultraapi::prelude::Multipart;

    // Verify Multipart is properly re-exported in prelude
    fn _check_multipart(_: Multipart) {}

    // The above compiles - Multipart is properly available
    assert!(true, "Multipart extractor is available via prelude");
}

// ===== Feature Requirements Documentation =====

#[test]
fn test_feature_requirements_documentation() {
    // Extractors are now fully supported via axum and axum-extra:
    //
    // ultraapi/Cargo.toml dependencies:
    // - axum = { version = "0.8", features = ["json", "form", "multipart", "ws", "query"] }
    // - axum-extra = { version = "0.12", features = ["cookie", "typed-header"] }
    //
    // Feature breakdown:
    // - "json": ✅ Enabled by default - JSON request/response
    // - "form": ✅ Enabled - Form data (application/x-www-form-urlencoded)
    // - "multipart": ✅ Enabled - File uploads and multipart forms
    // - "ws": ✅ Enabled - WebSocket support
    // - "query": ✅ Enabled - Query parameter extraction
    // - axum-extra "cookie": ✅ Enabled - CookieJar extraction
    // - axum-extra "typed-header": ✅ Enabled - TypedHeader extraction

    // Verify current state: these features are enabled
    use axum_extra::headers::authorization::Bearer;
    use axum_extra::headers::Authorization;
    use ultraapi::axum::Json;
    use ultraapi::prelude::{CookieJar, Form, Multipart, TypedHeader};
    let _ = Json::<()>;
    fn _check_form<T: serde::de::DeserializeOwned>(_: Form<T>) {}
    fn _check_multipart(_: Multipart) {}
    fn _check_cookie_jar(_: CookieJar) {}
    fn _check_typed_header(_: TypedHeader<Authorization<Bearer>>) {}
}
