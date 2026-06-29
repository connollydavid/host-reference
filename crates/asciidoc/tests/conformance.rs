//! Conformance fixture for the AsciiDoc normaliser, the same harness as the other readers. Never
//! auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_asciidoc::AsciidocNormalizer;

#[test]
fn asciidoc_guide_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "guide",
        "input.adoc",
        "adoc",
        &AsciidocNormalizer,
    );
}
