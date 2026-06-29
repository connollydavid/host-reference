//! The office normaliser: DOCX, PPTX, and XLSX through undoc's unified document model. The skeleton is
//! the document title, the section names (slides and sheets) and heading outline walked from the
//! sections, and the embedded-resource count. The full text view is a later refinement. The source
//! map is whole-document for now.

use host_reference_core::{Caps, Error, Modality, Normalizer, Semantic, Source, Tier0};
use undoc::{parse_bytes, Block, Paragraph};

pub struct OfficeNormalizer;

impl Normalizer for OfficeNormalizer {
    fn modality(&self) -> Modality {
        Modality::OfficeCompound
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["docx", "pptx", "xlsx"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let outline = office_shape(source.bytes)?;
        Ok(host_reference_core::Tier0::whole(source.bytes, outline))
    }
}

fn office_shape(bytes: &[u8]) -> Result<String, Error> {
    // undoc's zip layer decompresses every part without a cap, so refuse a decompression bomb
    // up front (finding 2 instance, call/0031).
    host_reference_core::decompression_guard("office", bytes)?;
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
