//! The per-kind round-trip law in the property-based lane (call/0030). For every kind that declares a
//! well-behaved lens (`write_back: true`), the lens laws hold over arbitrary text and arbitrary edits:
//!
//! - GetPut: writing a span back with its own current content changes nothing.
//! - PutGet: writing a replacement is exactly the splice, so the edit is reflected and only there.
//!
//! The write-back kinds today are prose and structured data; both carry the UTF-8 text-splice lens.

use host_reference_core::{content_id, Edit, Normalizer, Source, Span};
use host_reference_data::DataNormalizer;
use host_reference_prose::ProseNormalizer;
use proptest::prelude::*;

/// Floor an index to the nearest char boundary at or below it, the same clamp the lenses apply.
fn floor(text: &str, i: usize) -> usize {
    let mut i = i.min(text.len());
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn get_put(normalizer: &dyn Normalizer, hint: &str, text: &str, a: usize, b: usize) {
    let bytes = text.as_bytes();
    let source = Source { bytes, hint: Some(hint) };
    let (start, end) = (floor(text, a.min(b)), floor(text, a.max(b)));
    let edit = Edit {
        at: Span { source: content_id(bytes), origin: start..end },
        replacement: text[start..end].to_string(),
    };
    let patch = normalizer.put(&source, &edit).expect("put");
    assert_eq!(patch.bytes, bytes, "GetPut: a no-op edit must not change the source");
}

fn put_get(normalizer: &dyn Normalizer, hint: &str, text: &str, a: usize, b: usize, replacement: &str) {
    let bytes = text.as_bytes();
    let source = Source { bytes, hint: Some(hint) };
    let (start, end) = (floor(text, a.min(b)), floor(text, a.max(b)));
    let edit = Edit {
        at: Span { source: content_id(bytes), origin: start..end },
        replacement: replacement.to_string(),
    };
    let patch = normalizer.put(&source, &edit).expect("put");
    let expected = format!("{}{}{}", &text[..start], replacement, &text[end..]);
    assert_eq!(String::from_utf8(patch.bytes).unwrap(), expected, "PutGet: the edit is exactly the splice");
}

proptest! {
    #[test]
    fn prose_get_put(text in ".{0,80}", a in 0usize..120, b in 0usize..120) {
        get_put(&ProseNormalizer, "md", &text, a, b);
    }

    #[test]
    fn prose_put_get(text in ".{0,80}", a in 0usize..120, b in 0usize..120, r in ".{0,40}") {
        put_get(&ProseNormalizer, "md", &text, a, b, &r);
    }

    #[test]
    fn data_get_put(text in ".{0,80}", a in 0usize..120, b in 0usize..120) {
        get_put(&DataNormalizer, "csv", &text, a, b);
    }

    #[test]
    fn data_put_get(text in ".{0,80}", a in 0usize..120, b in 0usize..120, r in ".{0,40}") {
        put_get(&DataNormalizer, "csv", &text, a, b, &r);
    }
}
