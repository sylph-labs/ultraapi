// Compile-fail / compile-pass macro tests using trybuild

use trybuild::TestCases;

#[test]
fn test_compile_fail_extractors() {
    let t = TestCases::new();

    // Note: TypedHeader and CookieJar are now supported via axum-extra.
    // These legacy files are kept as compile-pass checks.
    t.pass("tests/ui/compile_fail_header_extractor.rs");
    t.pass("tests/ui/compile_fail_cookie_extractor.rs");
}

#[test]
fn test_response_model_parser_variants() {
    let t = TestCases::new();

    // Valid variants (order/whitespace/compound options)
    t.pass("tests/ui/response_model_parser_pass.rs");

    // Invalid variants should produce compile errors
    t.compile_fail("tests/ui/response_model_parser_invalid_unknown_option.rs");
    t.compile_fail("tests/ui/response_model_parser_invalid_bool.rs");
    t.compile_fail("tests/ui/response_model_parser_invalid_selector_syntax.rs");
}
