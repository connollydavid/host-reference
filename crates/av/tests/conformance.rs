//! Conformance fixtures for the audio-visual normaliser, the same harness as the other readers.
//! Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_av::AvNormalizer;

#[test]
fn wav_audio_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "tone",
        "input.wav",
        "wav",
        &AvNormalizer,
    );
}

#[test]
fn mp4_video_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "clip",
        "input.mp4",
        "mp4",
        &AvNormalizer,
    );
}
