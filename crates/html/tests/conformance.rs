//! Conformance fixture for the HTML normaliser, the same harness pattern: run the normaliser,
//! serialise tier-0 canonically, assert it equals the committed golden byte for byte. Never
//! auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_html::HtmlNormalizer;

#[test]
fn html5_page_outline() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "page",
        "input.html",
        "html",
        &HtmlNormalizer,
    );
}
