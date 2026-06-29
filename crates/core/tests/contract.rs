//! The Normalizer-contract laws of host-reference.allium, exercised at the trait level. These name the
//! behavioural obligations the spec's rules carry (Normalise, Window, WriteBack, WriteBackRefused) and
//! discharge them against the trait, independent of any one reader. The per-format conformance fixtures
//! and the overlay lens-law proptests cover the instances; these cover the contract.

use host_reference_core::{
    content_id, Caps, Edit, Error, Modality, Normalizer, Patch, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

/// A minimal text reader that declares a well-behaved lens: a stand-in for any write-back kind.
struct DocReader;

impl Normalizer for DocReader {
    fn modality(&self) -> Modality {
        Modality::Prose
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: true, write_back: true, semantic: Semantic::None, ocr: false }
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[]
    }

    fn detect(&self, source: &Source) -> bool {
        !matches!(source.hint, None | Some(""))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(e.to_string()))?;
        Ok(Tier0 {
            markdown: text.to_string(),
            source_map: SourceMap {
                spans: vec![Span {
                    source: content_id(source.bytes),
                    origin: 0..source.bytes.len(),
                }],
            },
            raw_tokens: text.len(),
            normalised_tokens: text.len(),
        })
    }

    fn view(&self, source: &Source, select: &SpanSelector) -> Result<Tier1, Error> {
        let text = std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(e.to_string()))?;
        match select {
            SpanSelector::CharOffset { start, len } => {
                let s = (*start).min(text.len());
                let e = (s + *len).min(text.len());
                Ok(Tier1 {
                    markdown: text[s..e].to_string(),
                    source_map: SourceMap {
                        spans: vec![Span { source: content_id(source.bytes), origin: s..e }],
                    },
                })
            }
            _ => Err(Error::Unsupported("view selector")),
        }
    }

    fn put(&self, source: &Source, edit: &Edit) -> Result<Patch, Error> {
        let text = std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(e.to_string()))?;
        let (start, end) =
            (edit.at.origin.start.min(text.len()), edit.at.origin.end.min(text.len()));
        let mut out = String::new();
        out.push_str(&text[..start]);
        out.push_str(&edit.replacement);
        out.push_str(&text[end..]);
        Ok(Patch { bytes: out.into_bytes() })
    }
}

/// A read-only reader: it declares no lens, so it takes the trait's default `put`, the fail-safe.
struct ReadOnly;

impl Normalizer for ReadOnly {
    fn modality(&self) -> Modality {
        Modality::FixedLayout
    }
    fn capabilities(&self) -> Caps {
        Caps::default()
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["ro"]
    }
    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        Ok(Tier0 {
            source_map: SourceMap {
                spans: vec![Span {
                    source: content_id(source.bytes),
                    origin: 0..source.bytes.len(),
                }],
            },
            ..Default::default()
        })
    }
    fn view(&self, _source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
        Err(Error::Unsupported("view"))
    }
    // put is the trait default: it refuses.
}

/// A reader that enforces a resource bound, the call/0031 fail-safe at the trait level. An input
/// past the bound is hostile and is refused explicitly, never parsed into a silent partial or
/// allowed to panic. A real reader discovers hostility while parsing (an overflowing selector, a
/// malformed structure, a decompression bomb); this stand-in models the contract the rules name,
/// independent of any one format. The `hostile`/`trusted` attributes the spec carries on Source are
/// realised here as "the bytes trip the reader's bound".
struct GuardedReader;

const GUARD_BOUND: usize = 16;

impl Normalizer for GuardedReader {
    fn modality(&self) -> Modality {
        Modality::Prose
    }
    fn capabilities(&self) -> Caps {
        Caps::default()
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["guarded"]
    }
    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        if source.bytes.len() > GUARD_BOUND {
            return Err(Error::Refused("over the size bound".into()));
        }
        Ok(Tier0 {
            source_map: SourceMap {
                spans: vec![Span {
                    source: content_id(source.bytes),
                    origin: 0..source.bytes.len(),
                }],
            },
            ..Default::default()
        })
    }
    fn view(&self, source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
        if source.bytes.len() > GUARD_BOUND {
            return Err(Error::Refused("over the size bound".into()));
        }
        Ok(Tier1::default())
    }
}

fn src<'a>(bytes: &'a [u8], hint: &'a str) -> Source<'a> {
    Source { bytes, hint: Some(hint) }
}

#[test]
fn normalise_yields_attested_skeleton() {
    // Normalise: a reader derives a skeleton, and every region carries an origin span (the
    // bidirectional source map), so the immutable layer is traceable.
    let bytes = b"hello world";
    let t0 = DocReader.skeleton(&src(bytes, "md")).expect("skeleton");
    assert!(!t0.source_map.spans.is_empty());
    assert_eq!(t0.markdown, "hello world");
}

#[test]
fn normalise_requires_a_format_hint() {
    // Normalise requires a hint: a reader does not claim a source it cannot place.
    assert!(!DocReader.detect(&Source { bytes: b"x", hint: None }));
    assert!(!DocReader.detect(&Source { bytes: b"x", hint: Some("") }));
    assert!(DocReader.detect(&src(b"x", "md")));
}

#[test]
fn window_yields_a_slice() {
    // Window: a windowed slice is fetched on demand, costing only the slice the task needs.
    let bytes = b"hello world";
    let t1 = DocReader
        .view(&src(bytes, "md"), &SpanSelector::CharOffset { start: 0, len: 5 })
        .expect("view");
    assert_eq!(t1.markdown, "hello");
    assert!(!t1.source_map.spans.is_empty());
}

#[test]
fn window_refuses_an_unsupported_selector() {
    // The reverse of the success path: an unsupported selector refuses rather than guesses.
    let err = DocReader.view(&src(b"hello", "md"), &SpanSelector::Section("nope".into()));
    assert!(matches!(err, Err(Error::Unsupported(_))));
}

#[test]
fn write_back_produces_a_patch() {
    // WriteBack: where a reader declares a well-behaved lens, an edit writes back as a patch.
    let bytes = b"hello world";
    let edit = Edit {
        at: Span { source: content_id(bytes), origin: 0..5 },
        replacement: "goodbye".to_string(),
    };
    let patch = DocReader.put(&src(bytes, "md"), &edit).expect("put");
    assert_eq!(patch.bytes, b"goodbye world");
}

#[test]
fn write_back_refused_without_the_capability() {
    // WriteBackRefused: a reader that declares no lens takes the default put, which refuses. The
    // fail-safe: a kind that has not earned editability cannot be edited through.
    let bytes = b"opaque";
    let edit =
        Edit { at: Span { source: content_id(bytes), origin: 0..1 }, replacement: "x".into() };
    assert!(matches!(ReadOnly.put(&src(bytes, "ro"), &edit), Err(Error::Unsupported(_))));
}

#[test]
fn refuse_hostile_input_yields_refused() {
    // RefuseHostileInput: an untrusted source that trips a resource bound is refused explicitly
    // (Error::Refused), never a panic or a silent partial; a benign source is normalised, so the
    // rule fires only on the hostile case.
    let hostile = vec![b'x'; 64];
    assert!(matches!(GuardedReader.skeleton(&src(&hostile, "guarded")), Err(Error::Refused(_))));
    assert!(GuardedReader.skeleton(&src(b"small", "guarded")).is_ok());
}

#[test]
fn window_refuses_a_hostile_source() {
    // rule-failure.Window.1: Window requires the source not be hostile, so a hostile source refuses
    // rather than returning a View. (Distinct from refusing an unsupported selector above.)
    let hostile = vec![b'x'; 64];
    let got = GuardedReader
        .view(&src(&hostile, "guarded"), &SpanSelector::CharOffset { start: 0, len: 4 });
    assert!(matches!(got, Err(Error::Refused(_))));
}
