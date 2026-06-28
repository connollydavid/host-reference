//! The reStructuredText normaliser: the skeleton is the section heading outline, walked from the
//! parsed document tree. rst_parser covers a README-level subset, so the outline reflects that; the
//! full text round-trips through the view. The source map is whole-document for now.

use document_tree::element_categories::{StructuralSubElement, SubStructure, TextOrInlineElement};
use document_tree::elements::Section;
use document_tree::HasChildren;
use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct RstNormalizer;

impl RstNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for RstNormalizer {
    fn modality(&self) -> Modality {
        Modality::Prose
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: true, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("rst"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let outline = rst_shape(text)?;
        Ok(Tier0 {
            raw_tokens: count_tokens(text),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }

    fn view(&self, source: &Source, select: &SpanSelector) -> Result<Tier1, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let (start, end) = match select {
            SpanSelector::CharOffset { start, len } => {
                host_reference_core::char_offset_window(text, *start, *len)
            }
            _ => (0, text.len()),
        };
        Ok(Tier1 {
            markdown: text[start..end].to_string(),
            source_map: SourceMap { spans: vec![Span { source: id, origin: start..end }] },
        })
    }
}

fn rst_shape(text: &str) -> Result<String, Error> {
    let doc = rst_parser::parse(text).map_err(|e| Error::Parse(format!("rst: {e}")))?;
    let mut out = String::new();
    walk(doc.children(), 0, &mut out);
    if out.is_empty() {
        out.push_str("(no sections)\n");
    }
    Ok(out)
}

fn walk(children: &[StructuralSubElement], depth: usize, out: &mut String) {
    for child in children {
        if let StructuralSubElement::SubStructure(sub) = child {
            if let SubStructure::Section(section) = sub.as_ref() {
                out.push_str(&format!("{}- {}\n", "  ".repeat(depth), section_title(section)));
                walk(section.children(), depth + 1, out);
            }
        }
    }
}

fn section_title(section: &Section) -> String {
    for c in section.children() {
        if let StructuralSubElement::Title(title) = c {
            return inline_text(title.children());
        }
    }
    "(untitled)".to_string()
}

fn inline_text(children: &[TextOrInlineElement]) -> String {
    let mut s = String::new();
    for c in children {
        if let TextOrInlineElement::String(t) = c {
            s.push_str(t.as_str());
        }
    }
    s
}
