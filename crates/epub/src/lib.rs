//! The EPUB normaliser: an EPUB is a zip of XHTML, so the skeleton is the table-of-contents outline
//! (the nav, nested) with the book title, read through rbook from the in-memory bytes. Per-chapter
//! text through htmd is a later view refinement. The source map is whole-document for now.

use std::io::Cursor;

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};
use rbook::ebook::toc::TocEntry;
use rbook::Epub;

pub struct EpubNormalizer;

impl Normalizer for EpubNormalizer {
    fn modality(&self) -> Modality {
        Modality::Prose
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("epub"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = epub_outline(source.bytes)?;
        let lossy = String::from_utf8_lossy(source.bytes);
        Ok(Tier0 {
            raw_tokens: count_tokens(&lossy),
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
            markdown: epub_outline(source.bytes)?,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn epub_outline(bytes: &[u8]) -> Result<String, Error> {
    // rbook's zip layer decompresses without a cap; refuse a decompression bomb up front
    // (finding 2 instance, call/0031).
    host_reference_core::decompression_guard("epub", bytes)?;
    let epub = Epub::read(Cursor::new(bytes.to_vec())).map_err(|e| Error::Parse(format!("epub: {e}")))?;
    let mut out = String::new();
    if let Some(title) = epub.metadata().title() {
        out.push_str(&format!("title: {}\n", title.value()));
    }
    let toc = epub.toc();
    let mut entries: Vec<(usize, String)> = Vec::new();
    for root in toc.iter() {
        collect(&root, &mut entries);
        for descendant in root.flatten() {
            collect(&descendant, &mut entries);
        }
    }
    let base = entries.iter().map(|(d, _)| *d).min().unwrap_or(0);
    for (depth, label) in entries {
        out.push_str(&format!("{}- {}\n", "  ".repeat(depth - base), label));
    }
    Ok(out)
}

/// Collect an entry's depth and label, skipping the unlabelled container the nav wraps the list in.
fn collect<'a>(entry: &impl TocEntry<'a>, entries: &mut Vec<(usize, String)>) {
    let label = entry.label().trim();
    if !label.is_empty() {
        entries.push((entry.depth(), label.to_string()));
    }
}
