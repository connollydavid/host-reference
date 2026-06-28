//! The conformance-fixture harness, the pattern every normaliser follows. A fixture is a directory
//! under `fixtures/` with an `input.<ext>` and an `expected.golden`. The test runs the normaliser,
//! serialises the tier-0 canonically, and asserts it equals the golden byte for byte.
//!
//! A golden is never auto-blessed: set `HOST_REFERENCE_BLESS=1` to (re)write it deliberately, then
//! review the diff before committing. The default run fails loud on any drift.

use std::fs;
use std::path::Path;

use host_reference_core::{serialize_tier0, Normalizer, Source};
use host_reference_prose::ProseNormalizer;

fn check_fixture(dir: &str, input: &str, hint: &str) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(dir);
    let bytes = fs::read(base.join(input)).expect("read fixture input");
    let tier0 = ProseNormalizer
        .skeleton(&Source { bytes: &bytes, hint: Some(hint) })
        .expect("skeleton");
    let got = serialize_tier0(&tier0);

    let golden = base.join("expected.golden");
    if std::env::var("HOST_REFERENCE_BLESS").is_ok() {
        fs::write(&golden, &got).expect("write golden");
        return;
    }
    let want = fs::read_to_string(&golden)
        .expect("read golden; bless it first with HOST_REFERENCE_BLESS=1");
    assert_eq!(got, want, "tier-0 drifted from the golden for fixture `{dir}`");
}

#[test]
fn basic_markdown_with_a_chinese_section() {
    check_fixture("basic", "input.md", "md");
}
