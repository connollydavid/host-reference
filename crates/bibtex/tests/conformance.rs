//! Conformance fixture for the BibTeX normaliser, the same harness as the other readers. Never
//! auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_bibtex::BibtexNormalizer;

#[test]
fn bibtex_refs_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "refs",
        "input.bib",
        "bib",
        &BibtexNormalizer,
    );
}
