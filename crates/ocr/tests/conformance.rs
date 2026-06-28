//! Conformance for the OCR normaliser. The real engine lives out-of-process in the separate
//! `host-reference-ocr` repo and is conformance-tested there; this crate is permissive and carries no
//! engine. So the plugin's own contract, the out-of-process plumbing and the skeleton formatting, is
//! tested against a stub helper that emits fixed recognised text. The stub asserts it received a real
//! image path, so the test still proves the plugin stages the image and runs the helper at arm's
//! length. Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use std::fs;
use std::path::{Path, PathBuf};

use host_reference_core::{serialize_tier0, Normalizer, Source};
use host_reference_ocr::OcrNormalizer;

/// Write a stub helper that stands in for `host-reference-ocr-helper`: it checks an image path was
/// passed and prints fixed recognised text, so the plugin's plumbing is exercised without the engine.
fn write_stub() -> PathBuf {
    let stub = std::env::temp_dir().join("host-reference-ocr-stub.sh");
    fs::write(&stub, "#!/bin/sh\n[ \"$1\" = --version ] && { echo 'host-reference-ocr-helper stub'; exit 0; }\n[ -f \"$1\" ] || { echo 'stub: no image' >&2; exit 1; }\necho 'HELLO WORLD'\n")
        .expect("write stub helper");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    }
    stub
}

fn check(dir: &str, input: &str, hint: &str) {
    std::env::set_var("HOST_REFERENCE_OCR_HELPER", write_stub());
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(dir);
    let bytes = fs::read(base.join(input)).expect("read fixture input");
    let tier0 =
        OcrNormalizer.skeleton(&Source { bytes: &bytes, hint: Some(hint) }).expect("skeleton");
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
fn scan_formats_helper_text() {
    check("scan", "input.png", "png");
}
