// Compile-fail tests using trybuild
// These tests verify that certain extractor types are NOT handled by UltraAPI macro
// The tests/ui/*.rs files should fail to compile when used with the UltraAPI macro

use trybuild::TestCases;

#[test]
fn test_compile_fail_extractors() {
    let t = TestCases::new();

    // Note: TypedHeader and CookieJar are now supported via axum-extra
    // Previously these were compile-fail tests but now they work
    // The tests have been moved to extractor_tests.rs as passing tests

    // This test file verifies that the old compile-fail tests are now passing
    // If you need to add new compile-fail tests, add them to tests/ui/
    t.pass("tests/ui/compile_fail_header_extractor.rs");
    t.pass("tests/ui/compile_fail_cookie_extractor.rs");
}
