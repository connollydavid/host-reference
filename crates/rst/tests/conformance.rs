//! Conformance fixture for the reStructuredText normaliser, the same harness as the other readers.
//! Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_rst::RstNormalizer;

#[test]
fn rst_guide_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "guide",
        "input.rst",
        "rst",
        &RstNormalizer,
    );
}

// A section title with inline markup (literal, emphasis, strong); the outline must keep the full
// title text rather than dropping the marked-up runs.
#[test]
fn rst_title_with_inline_markup() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "markup",
        "input.rst",
        "rst",
        &RstNormalizer,
    );
}
