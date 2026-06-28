//! Conformance for the OCR normaliser. Unlike the in-process readers it drives the out-of-process
//! helper, so the test makes sure the helper binary is built and points the plugin at it through the
//! same environment variable a real deployment uses. Never auto-blessed; set `HOST_REFERENCE_BLESS=1`
//! to rewrite a golden deliberately.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use host_reference_core::{serialize_tier0, Normalizer, Source};
use host_reference_ocr::OcrNormalizer;

fn ensure_helper() -> PathBuf {
    let helper = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/host-reference-ocr-helper");
    if !helper.exists() {
        let status = Command::new(env!("CARGO"))
            .args(["build", "-p", "host-reference-ocr-helper"])
            .status()
            .expect("run cargo build for the ocr helper");
        assert!(status.success(), "ocr helper build failed");
    }
    helper.canonicalize().unwrap_or(helper)
}

fn check(dir: &str, input: &str, hint: &str) {
    std::env::set_var("HOST_REFERENCE_OCR_HELPER", ensure_helper());
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(dir);
    let bytes = fs::read(base.join(input)).expect("read fixture input");
    let tier0 = OcrNormalizer
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
fn scan_hello_world_shape() {
    check("scan", "input.png", "png");
}
