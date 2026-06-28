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

    r1.merge(&r2.export().unwrap()).unwrap();
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
