//! Conformance fixture for the Org-mode normaliser, the same harness as the other readers. Never
//! auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_org::OrgNormalizer;

#[test]
fn org_notes_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "notes",
        "input.org",
        "org",
        &OrgNormalizer,
    );
}
