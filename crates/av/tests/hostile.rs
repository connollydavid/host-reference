//! call/0031 hostile-input coverage for the av reader: a PCM WAV whose declared data-chunk size
//! exceeds what the file can hold is refused rather than yielding a fabricated duration (plan/0050).

use host_reference_av::AvNormalizer;
use host_reference_core::{Error, Normalizer, Source};

/// A canonical 16-bit mono PCM WAV header whose `data` chunk declares `declared` bytes, followed by
/// `actual` real sample bytes. A bogus large `declared` is the hostile case.
fn wav(declared: u32, actual: &[u8]) -> Vec<u8> {
    let mut w = Vec::new();
    w.extend_from_slice(b"RIFF");
    // RIFF size covers the declared data chunk, so symphonia accepts the structure and reads the
    // bogus frame count from the data header; the actual bytes are far shorter.
    w.extend_from_slice(&(36u32 + declared).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&8000u32.to_le_bytes()); // sample rate
    w.extend_from_slice(&16000u32.to_le_bytes()); // byte rate
    w.extend_from_slice(&2u16.to_le_bytes()); // block align
    w.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    w.extend_from_slice(b"data");
    w.extend_from_slice(&declared.to_le_bytes());
    w.extend_from_slice(actual);
    w
}

#[test]
fn refuses_a_bogus_wav_chunk() {
    // The data chunk declares 100 MB but the file carries 8 bytes of samples; the declared frame
    // count cannot be backed by the bytes, so the reader refuses instead of fabricating a duration.
    let bytes = wav(100_000_000, &[0u8; 8]);
    let got = AvNormalizer.skeleton(&Source { bytes: &bytes, hint: Some("wav") });
    assert!(matches!(got, Err(Error::Refused(_))), "expected a refusal, got {got:?}");
}
