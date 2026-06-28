//! The overlay: the mutable, collaborative layer over the immutable normalised layer (call/0030). It
//! is a Loro CRDT document holding annotations, edits, and notes, each anchored to the immutable layer
//! by a W3C Web Annotation selector. A `TextQuote` selector re-locates by content, so an annotation
//! survives a re-derivation that shifts offsets. The write-back path resolves a selector to a span and
//! drives the normaliser's `put`, the well-behaved lens, where one exists; the round-trip law of that
//! lens is proptested per kind in the property-based lane (the crate's tests).

use std::ops::Range;

use host_reference_core::{content_id, Edit, Error, Normalizer, Patch, Source, Span};
use loro::{ExportMode, LoroDoc, LoroValue};
use serde::{Deserialize, Serialize};

/// A W3C Web Annotation selector, the standard anchor into the immutable layer. `TextPosition` is the
/// offset pair; `TextQuote` carries the matched text with a little context, so it re-locates across a
/// re-derivation that shifts offsets.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    TextPosition { start: usize, end: usize },
    TextQuote { prefix: String, exact: String, suffix: String },
}

/// One overlay entry: a body anchored to a source by a selector. The source is the content id, the
/// stable key the immutable layer hangs on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    pub source: String,
    pub selector: Selector,
    pub body: String,
}

/// Resolve a selector to a span in `text`. `TextPosition` is bounds-and-boundary checked;
/// `TextQuote` looks for the exact text in its prefix/suffix context, then falls back to the bare
/// quote, which is what lets it survive a re-derivation.
pub fn resolve(selector: &Selector, text: &str) -> Option<Range<usize>> {
    match selector {
        Selector::TextPosition { start, end } => {
            if start <= end
                && *end <= text.len()
                && text.is_char_boundary(*start)
                && text.is_char_boundary(*end)
            {
                Some(*start..*end)
            } else {
                None
            }
        }
        Selector::TextQuote { prefix, exact, suffix } => {
            let contextual = format!("{prefix}{exact}{suffix}");
            if let Some(i) = text.find(&contextual) {
                let s = i + prefix.len();
                return Some(s..s + exact.len());
            }
            text.find(exact.as_str()).map(|i| i..i + exact.len())
        }
    }
}

/// The mutable overlay, a Loro document. Annotations are stored as JSON in a Loro list, so concurrent
/// additions merge at the entry granularity; `export`/`import` persist a snapshot and `merge` folds in
/// another replica's snapshot.
pub struct Overlay {
    doc: LoroDoc,
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Overlay {
    pub fn new() -> Self {
        Overlay { doc: LoroDoc::new() }
    }

    /// A replica with an explicit peer id. Each replica in a collaborative setting needs a distinct
    /// peer so its edits are distinguishable when the CRDT merges; `new` takes a random peer.
    pub fn with_peer(peer: u64) -> Result<Self, Error> {
        let doc = LoroDoc::new();
        doc.set_peer_id(peer).map_err(|e| Error::Parse(format!("overlay: {e}")))?;
        Ok(Overlay { doc })
    }

    /// Add an annotation.
    pub fn annotate(&self, annotation: &Annotation) -> Result<(), Error> {
        let json =
            serde_json::to_string(annotation).map_err(|e| Error::Parse(format!("overlay: {e}")))?;
        self.doc
            .get_list("annotations")
            .push(json)
            .map_err(|e| Error::Parse(format!("overlay: {e}")))?;
        self.doc.commit();
        Ok(())
    }

    /// Read every annotation currently in the overlay.
    pub fn annotations(&self) -> Vec<Annotation> {
        let list = self.doc.get_list("annotations");
        let mut out = Vec::new();
        for i in 0..list.len() {
            if let Some(LoroValue::String(s)) = list.get(i).and_then(|v| v.into_value().ok()) {
                if let Ok(a) = serde_json::from_str::<Annotation>(&s) {
                    out.push(a);
                }
            }
        }
        out
    }

    /// A persistable snapshot of the overlay.
    pub fn export(&self) -> Result<Vec<u8>, Error> {
        self.doc
            .export(ExportMode::Snapshot)
            .map_err(|e| Error::Parse(format!("overlay export: {e}")))
    }

    /// Load an overlay from a snapshot.
    pub fn import(bytes: &[u8]) -> Result<Self, Error> {
        let doc = LoroDoc::new();
        doc.import(bytes).map_err(|e| Error::Parse(format!("overlay import: {e}")))?;
        Ok(Overlay { doc })
    }

    /// Fold another replica's snapshot into this overlay; the CRDT merges the two.
    pub fn merge(&self, snapshot: &[u8]) -> Result<(), Error> {
        self.doc.import(snapshot).map_err(|e| Error::Parse(format!("overlay merge: {e}")))?;
        Ok(())
    }
}

/// The write-back path over a selector: resolve the selector against the source, build the `Edit`, and
/// drive the normaliser's `put`. The normaliser refuses unless it declares a well-behaved lens, the
/// fail-safe of `call/0030`.
pub fn write_back(
    normalizer: &dyn Normalizer,
    source: &Source,
    selector: &Selector,
    replacement: &str,
) -> Result<Patch, Error> {
    let text = std::str::from_utf8(source.bytes)
        .map_err(|e| Error::Parse(format!("write-back: source is not UTF-8: {e}")))?;
    let range = resolve(selector, text)
        .ok_or_else(|| Error::Parse("write-back: selector did not resolve".to_string()))?;
    let edit = Edit {
        at: Span { source: content_id(source.bytes), origin: range },
        replacement: replacement.to_string(),
    };
    normalizer.put(source, &edit)
}
