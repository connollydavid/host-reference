//! Conformance fixture for the RTF normaliser, the same harness as the other readers. Never
//! auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_rtf::RtfNormalizer;

#[test]
fn rtf_letter_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "letter",
        "input.rtf",
        "rtf",
        &RtfNormalizer,
    );
}
