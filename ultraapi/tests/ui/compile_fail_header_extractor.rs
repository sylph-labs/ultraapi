// This file tests that TypedHeader extractor IS now handled by UltraAPI macro
// With axum-extra and the typed-header feature, this now works

use ultraapi::prelude::*;
use axum_extra::extract::TypedHeader;
use axum_extra::headers::Authorization;

// This should now work - TypedHeader is handled by the UltraAPI macro via axum-extra
#[get("/test-header")]
async fn test_handler(
    _auth: TypedHeader<Authorization<axum_extra::headers::authorization::Bearer>>,
) -> String {
    "works".to_string()
}

fn main() {}
