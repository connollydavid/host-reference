//! The conformance-fixture harness, the pattern every normaliser follows. A fixture is a directory
//! under `fixtures/` with an `input.<ext>` and an `expected.golden`. The test runs the normaliser,
//! serialises the tier-0 canonically, and asserts it equals the golden byte for byte.
//!
//! A golden is never auto-blessed: set `HOST_REFERENCE_BLESS=1` to (re)write it deliberately, then
//! review the diff before committing. The default run fails loud on any drift.

use host_reference_prose::ProseNormalizer;

#[test]
fn basic_markdown_with_a_chinese_section() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "basic",
        "input.md",
        "md",
        &ProseNormalizer,
    );
}
