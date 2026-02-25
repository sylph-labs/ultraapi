// This file tests that CookieJar extractor IS now handled by UltraAPI macro
// With axum-extra and the cookie feature, this now works

use ultraapi::prelude::*;
use axum_extra::extract::CookieJar;

// This should now work - CookieJar is handled by the UltraAPI macro via axum-extra
#[get("/test-cookie")]
async fn test_handler(
    _cookies: CookieJar,
) -> String {
    "works".to_string()
}

fn main() {}
