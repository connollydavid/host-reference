//! Overlay behaviour: annotations persist across an export/import, the CRDT merges concurrent
//! replicas, and a TextQuote selector survives a re-derivation that shifts offsets.

use host_reference_overlay::{resolve, Annotation, Overlay, Selector};

fn ann(body: &str, selector: Selector) -> Annotation {
    Annotation { source: "0a1b2c3d4e5f".to_string(), selector, body: body.to_string() }
}

#[test]
fn annotate_export_import_roundtrips() {
    let overlay = Overlay::new();
    let a = ann("a margin note", Selector::TextPosition { start: 0, end: 5 });
    overlay.annotate(&a).unwrap();

    let snapshot = overlay.export().unwrap();
    let loaded = Overlay::import(&snapshot).unwrap();
    assert_eq!(loaded.annotations(), vec![a]);
}

#[test]
fn crdt_merge_unions_concurrent_annotations() {
    // Two replicas with distinct peers each add an annotation; merging unions them.
    let r1 = Overlay::with_peer(1).unwrap();
    let r2 = Overlay::with_peer(2).unwrap();
    let a1 = ann("from replica one", Selector::TextPosition { start: 0, end: 3 });
    let a2 = ann("from replica two", Selector::TextPosition { start: 7, end: 9 });
    r1.annotate(&a1).unwrap();
    r2.annotate(&a2).unwrap();

    // Merge symmetrically and assert the two replicas converge to the identical sequence, not
    // merely the same set: a non-convergent or order-divergent merge would be a CRDT defect the
    // one-direction set-membership check could not catch (finding 11).
    r1.merge(&r2.export().unwrap()).unwrap();
    r2.merge(&r1.export().unwrap()).unwrap();
    assert_eq!(r1.annotations(), r2.annotations(), "replicas converge to the same sequence");
    let bodies: Vec<String> = r1.annotations().into_iter().map(|a| a.body).collect();
    assert_eq!(bodies.len(), 2);
    assert!(bodies.contains(&"from replica one".to_string()));
    assert!(bodies.contains(&"from replica two".to_string()));
}

#[test]
fn text_quote_survives_re_derivation() {
    // The same quote is found after a re-derivation collapses the surrounding whitespace and shifts
    // every offset, which a bare TextPosition could not follow.
    let original = "intro paragraph\n\n   the load-bearing claim   \n\ntrailing";
    let rederived = "intro paragraph\nthe load-bearing claim\ntrailing";
    let selector = Selector::TextQuote {
        prefix: String::new(),
        exact: "the load-bearing claim".to_string(),
        suffix: String::new(),
    };

    let a = resolve(&selector, original).expect("resolves in the original");
    let b = resolve(&selector, rederived).expect("still resolves after re-derivation");
    assert_ne!(a.start, b.start, "the offset shifted");
    assert_eq!(&original[a], "the load-bearing claim");
    assert_eq!(&rederived[b], "the load-bearing claim");
}

#[test]
fn text_position_is_bounds_checked() {
    let text = "short";
    assert_eq!(resolve(&Selector::TextPosition { start: 0, end: 5 }, text), Some(0..5));
    assert_eq!(resolve(&Selector::TextPosition { start: 0, end: 99 }, text), None);
}

#[test]
fn text_quote_refuses_an_ambiguous_bare_match() {
    // Finding 5: the quote "cat" appears twice with no disambiguating context, so the bare match is
    // ambiguous. The old reader silently re-anchored to the first; resolution now refuses.
    let text = "on the cat, off the cat";
    let bare =
        Selector::TextQuote { prefix: String::new(), exact: "cat".into(), suffix: String::new() };
    assert_eq!(resolve(&bare, text), None);
}

#[test]
fn text_quote_uses_context_to_disambiguate() {
    // The same ambiguous quote resolves to the intended occurrence when its context matches, the
    // property a TextQuote selector exists to provide.
    let text = "on the cat, off the cat";
    let with_ctx = Selector::TextQuote {
        prefix: "off the ".into(),
        exact: "cat".into(),
        suffix: String::new(),
    };
    let r = resolve(&with_ctx, text).expect("the contextual match resolves");
    assert_eq!(&text[r.clone()], "cat");
    assert_eq!(r.start, 20, "it anchors to the second cat, not the first");
}

#[test]
fn text_quote_refuses_an_empty_quote() {
    // An empty exact would make str::find return 0; the degenerate selector resolves to nothing.
    let empty =
        Selector::TextQuote { prefix: String::new(), exact: String::new(), suffix: String::new() };
    assert_eq!(resolve(&empty, "any text"), None);
}
