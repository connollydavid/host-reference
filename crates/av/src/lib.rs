//! The audio-visual normaliser: deterministic, attested container and codec metadata only
//! (call/0032). Audio (WAV, FLAC, Ogg) is probed by symphonia for codec, sample rate, channels, and
//! duration; video containers (MP4 and kin) are read by the mp4 crate for per-track type, dimensions,
//! and duration. It does NOT transcribe: speech-to-text is non-deterministic and rides the overlay
//! adapter (call/0030), not this attested reader. The source map is whole-document.

use std::io::Cursor;

use host_reference_core::{
    content_id, count_tokens, guard_panic, Caps, Error, Modality, Normalizer, Semantic, Source,
    SourceMap, Span, SpanSelector, Tier0, Tier1,
};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AvNormalizer;

impl Normalizer for AvNormalizer {
    fn modality(&self) -> Modality {
        Modality::AudioVisual
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::None, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(
            source.hint,
            Some("wav" | "flac" | "ogg" | "oga" | "mp4" | "mov" | "m4v" | "m4a")
        )
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = shape(source)?;
        Ok(Tier0 {
            raw_tokens: source.bytes.len(),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }

    fn view(&self, source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
        let id = content_id(source.bytes);
        Ok(Tier1 {
            markdown: shape(source)?,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn shape(source: &Source) -> Result<String, Error> {
    match source.hint {
        // mp4 0.14 can under/overflow on a box whose declared size is smaller than its header or
        // larger than the file; the guard turns that unwind into a refusal (call/0031).
        Some("mp4" | "mov" | "m4v" | "m4a") => guard_panic("mp4", || mp4_shape(source.bytes)),
        _ => audio_shape(source.bytes, source.hint),
    }
}

fn audio_shape(bytes: &[u8], ext: Option<&str>) -> Result<String, Error> {
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = ext {
        hint.with_extension(e);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| Error::Parse(format!("audio: {e}")))?;
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| Error::Parse("audio: no track".to_string()))?;
    let params = &track.codec_params;
    let codec = symphonia::default::get_codecs()
        .get_codec(params.codec)
        .map(|d| d.short_name)
        .unwrap_or("unknown");
    // An uncompressed PCM WAV declares its sample count in the data-chunk header; a bogus size
    // (0xFFFFFFFF) would fabricate a duration the bytes cannot back. Refuse when the declared
    // frames exceed what the file can hold. Gated to PCM, where one frame maps to a fixed byte
    // count; a compressed codec has no such direct relation, so it is left alone (call/0031).
    if codec.starts_with("pcm") {
        if let (Some(frames), Some(bits), Some(channels)) =
            (params.n_frames, params.bits_per_sample, params.channels)
        {
            let frame_bytes = u64::from(bits / 8) * channels.count() as u64;
            if frame_bytes > 0 && frames > bytes.len() as u64 / frame_bytes {
                return Err(Error::Refused(format!(
                    "audio: declared {frames} frames exceed what the file can hold"
                )));
            }
        }
    }
    let mut out = format!("audio: {codec}");
    if let Some(rate) = params.sample_rate {
        out.push_str(&format!(", {rate} Hz"));
    }
    if let Some(channels) = params.channels {
        out.push_str(&format!(", {} ch", channels.count()));
    }
    if let (Some(frames), Some(rate)) = (params.n_frames, params.sample_rate) {
        if rate > 0 {
            let ms = u128::from(frames) * 1000 / u128::from(rate);
            out.push_str(&format!(", {ms} ms"));
        }
    }
    out.push('\n');
    Ok(out)
}

fn mp4_shape(bytes: &[u8]) -> Result<String, Error> {
    let reader = mp4::Mp4Reader::read_header(Cursor::new(bytes), bytes.len() as u64)
        .map_err(|e| Error::Parse(format!("mp4: {e}")))?;
    let mut out = format!("mp4: {} ms\n", reader.duration().as_millis());
    let mut tracks: Vec<(&u32, &mp4::Mp4Track)> = reader.tracks().iter().collect();
    tracks.sort_by_key(|(id, _)| **id);
    for (id, track) in tracks {
        let kind = track
            .track_type()
            .map(|t| format!("{t:?}").to_lowercase())
            .unwrap_or_else(|_| "unknown".to_string());
        let (w, h) = (track.width(), track.height());
        if w > 0 && h > 0 {
            out.push_str(&format!("- track {id}: {kind} {w}x{h}\n"));
        } else {
            out.push_str(&format!("- track {id}: {kind}\n"));
        }
    }
    Ok(out)
}
