//! The EPUB normaliser: an EPUB is a zip of XHTML, so the skeleton is the table-of-contents outline
//! (the nav, nested) with the book title, read through rbook from the in-memory bytes. Per-chapter
//! text through htmd is a later view refinement. The source map is whole-document for now.

use std::io::Cursor;

use host_reference_core::{Caps, Error, Modality, Normalizer, Semantic, Source, Tier0};
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

    fn extensions(&self) -> &'static [&'static str] {
        &["epub"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let outline = epub_outline(source.bytes)?;
        Ok(host_reference_core::Tier0::whole(source.bytes, outline))
    }
}

fn epub_outline(bytes: &[u8]) -> Result<String, Error> {
    // rbook's zip layer decompresses without a cap; refuse a decompression bomb up front
    // (finding 2 instance, call/0031).
    host_reference_core::decompression_guard("epub", bytes)?;
    let epub =
        Epub::read(Cursor::new(bytes.to_vec())).map_err(|e| Error::Parse(format!("epub: {e}")))?;
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
