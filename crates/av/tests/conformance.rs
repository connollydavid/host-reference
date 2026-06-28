//! Conformance fixtures for the audio-visual normaliser, the same harness as the other readers.
//! Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use std::fs;
use std::path::Path;

use host_reference_core::{serialize_tier0, Normalizer, Source};
use host_reference_av::AvNormalizer;

fn check(dir: &str, input: &str, hint: &str) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(dir);
    let bytes = fs::read(base.join(input)).expect("read fixture input");
    let tier0 = AvNormalizer
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
fn wav_audio_shape() {
    check("tone", "input.wav", "wav");
}

#[test]
fn mp4_video_shape() {
    check("clip", "input.mp4", "mp4");
}
