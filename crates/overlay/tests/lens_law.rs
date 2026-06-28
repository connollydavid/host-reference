//! The per-kind lens laws in the property-based lane (call/0030), exercised through the real read
//! and write sides, not a re-implemented oracle (plan/0050 finding 11). For every kind that declares
//! a well-behaved lens (`write_back: true`):
//!
//! - GetPut: writing a span back with the content `view` returns for it changes nothing.
//! - PutGet: after writing a replacement through `write_back`, `view` of the spliced region returns
//!   exactly that replacement.
//!
//! Both sides go through `host_reference_overlay::{resolve, write_back}` and the normaliser's `view`,
//! so the selector resolution and the splice are under test, not `put` in isolation, and the read
//! -back is a genuine `view`, not the splice formula re-applied as its own oracle. The generators
//! include newlines, the character that makes the prose structure non-trivial.
//!
//! The lens is well-behaved (GetPut and PutGet hold) but NOT very-well-behaved: PutPut does not hold
//! for a length-changing replacement, since a fixed-offset splice computes the second edit's offsets
//! against the original text. That boundary is asserted directly rather than claimed away.

use host_reference_core::{content_id, Edit, Normalizer, Source, Span, SpanSelector};
use host_reference_data::DataNormalizer;
use host_reference_overlay::{resolve, write_back, Selector};
use host_reference_prose::ProseNormalizer;
use proptest::prelude::*;

/// Floor an index to the nearest char boundary at or below it, so a selector built from arbitrary
/// offsets is valid.
fn floor(text: &str, i: usize) -> usize {
    let mut i = i.min(text.len());
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// GetPut through the real get (`view`) and put (`write_back` + `resolve`): reading a span and
/// writing it straight back is a no-op.
fn get_put(n: &dyn Normalizer, hint: &str, text: &str, a: usize, b: usize) {
    let bytes = text.as_bytes();
    let source = Source { bytes, hint: Some(hint) };
    let (lo, hi) = (floor(text, a.min(b)), floor(text, a.max(b)));
    let content = n
        .view(&source, &SpanSelector::CharOffset { start: lo, len: hi - lo })
        .expect("view")
        .markdown;
    let patch = write_back(n, &source, &Selector::TextPosition { start: lo, end: hi }, &content)
        .expect("write_back");
    assert_eq!(patch.bytes, bytes, "GetPut: writing a span back with its own content is a no-op");
}

/// PutGet through put (`write_back`) then get (`view`): after writing a replacement, viewing the
/// spliced region returns exactly the replacement. The read-back is a genuine `view`, so a get/put
/// mismatch would be caught.
fn put_get(n: &dyn Normalizer, hint: &str, text: &str, a: usize, b: usize, replacement: &str) {
    let bytes = text.as_bytes();
    let source = Source { bytes, hint: Some(hint) };
    let (lo, hi) = (floor(text, a.min(b)), floor(text, a.max(b)));
    let patch = write_back(n, &source, &Selector::TextPosition { start: lo, end: hi }, replacement)
        .expect("write_back");
    let new_source = Source { bytes: &patch.bytes, hint: Some(hint) };
    let got = n
        .view(&new_source, &SpanSelector::CharOffset { start: lo, len: replacement.len() })
        .expect("view")
        .markdown;
    assert_eq!(got, replacement, "PutGet: viewing the spliced region returns the replacement");
}

proptest! {
    #[test]
    fn prose_get_put(text in "[\\na-z .]{0,80}", a in 0usize..100, b in 0usize..100) {
        get_put(&ProseNormalizer, "md", &text, a, b);
    }

    #[test]
    fn prose_put_get(text in "[\\na-z .]{0,80}", a in 0usize..100, b in 0usize..100, r in "[\\na-z]{0,40}") {
        put_get(&ProseNormalizer, "md", &text, a, b, &r);
    }

    #[test]
    fn data_get_put(text in "[\\na-z,.]{0,80}", a in 0usize..100, b in 0usize..100) {
        get_put(&DataNormalizer, "csv", &text, a, b);
    }

    #[test]
    fn data_put_get(text in "[\\na-z,.]{0,80}", a in 0usize..100, b in 0usize..100, r in "[\\na-z]{0,40}") {
        put_get(&DataNormalizer, "csv", &text, a, b, &r);
    }
}

#[test]
fn lens_is_not_very_well_behaved_on_length_change() {
    // The documented boundary (finding 11): a fixed-offset splice violates PutPut when a replacement
    // changes the span length, because the second edit's offsets are computed against the original
    // text, not the result of the first. Two sequential edits do not equal one.
    let source = Source { bytes: b"abcdef", hint: Some("md") };
    let sel = Selector::TextPosition { start: 2, end: 4 };
    let first = write_back(&ProseNormalizer, &source, &sel, "X").unwrap();
    let then = Source { bytes: &first.bytes, hint: Some("md") };
    let twice = write_back(&ProseNormalizer, &then, &sel, "QQ").unwrap();
    let once = write_back(&ProseNormalizer, &source, &sel, "QQ").unwrap();
    assert_ne!(twice.bytes, once.bytes, "PutPut does not hold for a length-changing replacement");
}

#[test]
fn resolve_refuses_a_reversed_range() {
    // The reversed-range guard in resolve: start > end is not a valid position selector.
    assert_eq!(resolve(&Selector::TextPosition { start: 5, end: 2 }, "hello world"), None);
}

#[test]
fn put_tolerates_a_reversed_origin() {
    // The reversed-range guard in the lens put: a reversed origin collapses rather than panicking.
    let bytes = b"hello world";
    let (start, end) = (5usize, 2usize); // deliberately reversed; built from values so it is not a literal empty range
    let edit = Edit {
        at: Span { source: content_id(bytes), origin: start..end },
        replacement: "X".to_string(),
    };
    let patch = ProseNormalizer.put(&Source { bytes, hint: Some("md") }, &edit).expect("put");
    String::from_utf8(patch.bytes).expect("valid utf8");
}
