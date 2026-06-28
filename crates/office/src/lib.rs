//! The office normaliser, through undoc's unified document model. DOCX is wired and tested now; PPTX
//! and XLSX follow as their fixtures land (undoc reads all three). The skeleton is the document title,
//! the section names and heading outline walked from the sections, and the embedded-resource count.
//! The full text view is a later refinement. The source map is whole-document for now.

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};
use undoc::{parse_bytes, Block, Paragraph};

pub struct OfficeNormalizer;

impl Normalizer for OfficeNormalizer {
    fn modality(&self) -> Modality {
        Modality::OfficeCompound
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("docx"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = office_shape(source.bytes)?;
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
            markdown: office_shape(source.bytes)?,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn office_shape(bytes: &[u8]) -> Result<String, Error> {
    let doc = parse_bytes(bytes).map_err(|e| Error::Parse(format!("office: {e}")))?;
    let mut out = String::new();
    if let Some(title) = doc.metadata.title.as_deref() {
        out.push_str(&format!("title: {title}\n"));
    }
    for section in &doc.sections {
        let mut base = 0;
        if let Some(name) = section.name.as_deref().filter(|n| !n.is_empty()) {
            out.push_str(&format!("- {name}\n"));
            base = 1;
        }
        for block in &section.content {
            if let Block::Paragraph(p) = block {
                let level = heading_level(p);
                if level > 0 {
                    let depth = base + (level as usize - 1);
                    out.push_str(&format!("{}- {}\n", "  ".repeat(depth), p.plain_text()));
                }
            }
        }
    }
    out.push_str(&format!("resources: {}\n", doc.resources.len()));
    Ok(out)
}

/// The heading level of a paragraph: undoc's resolved level, or the Word heading style id
/// (`Heading1` through `Heading9`) when no `styles.xml` defines it. Zero means not a heading.
fn heading_level(p: &Paragraph) -> u8 {
    let resolved = p.heading.level();
    if resolved > 0 {
        return resolved;
    }
    p.style_id
        .as_deref()
        .and_then(|id| id.strip_prefix("Heading"))
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=9).contains(n))
        .unwrap_or(0)
}
