use host_reference_core::{serialize_tier0, Normalizer, Source};
use std::path::Path;

/// Run `normalizer` over `bytes` and assert its canonical tier-0 equals the committed golden at
/// `<manifest_dir>/fixtures/<dir>/expected.golden`. Rewritten only under HOST_REFERENCE_BLESS=1.
pub fn check_bytes<N: Normalizer>(
    manifest_dir: &str,
    dir: &str,
    bytes: &[u8],
    hint: &str,
    normalizer: &N,
) {
    let base = Path::new(manifest_dir).join("fixtures").join(dir);
    let tier0 = normalizer.skeleton(&Source { bytes, hint: Some(hint) }).expect("skeleton");
    let got = serialize_tier0(&tier0);
    let golden = base.join("expected.golden");
    if std::env::var("HOST_REFERENCE_BLESS").is_ok() {
        std::fs::create_dir_all(&base).expect("create fixture dir");
        std::fs::write(&golden, &got).expect("write golden");
        return;
    }
    let want = std::fs::read_to_string(&golden)
        .expect("read golden; bless it first with HOST_REFERENCE_BLESS=1");
    assert_eq!(got, want, "tier-0 drifted from the golden for fixture `{dir}`");
}

/// Like `check_bytes` but reads the input from `<manifest_dir>/fixtures/<dir>/<input>`.
pub fn check_file<N: Normalizer>(
    manifest_dir: &str,
    dir: &str,
    input: &str,
    hint: &str,
    normalizer: &N,
) {
    let bytes = std::fs::read(Path::new(manifest_dir).join("fixtures").join(dir).join(input))
        .expect("read fixture input");
    check_bytes(manifest_dir, dir, &bytes, hint, normalizer);
}
