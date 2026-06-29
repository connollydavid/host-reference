//! The reStructuredText normaliser: the skeleton is the section heading outline, walked from the
//! parsed document tree. rst_parser covers a README-level subset, so the outline reflects that; the
//! full text round-trips through the view. The source map is whole-document for now.

use document_tree::element_categories::{StructuralSubElement, SubStructure, TextOrInlineElement};
use document_tree::elements::Section;
use document_tree::HasChildren;
use host_reference_core::{
    content_id, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span, SpanSelector,
    Tier0, Tier1,
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

    fn extensions(&self) -> &'static [&'static str] {
        &["rst"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let outline = rst_shape(text)?;
        Ok(host_reference_core::Tier0::whole(source.bytes, outline))
    }

    fn view(&self, source: &Source, select: &SpanSelector) -> Result<Tier1, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        match select {
            SpanSelector::CharOffset { start, len } => {
                Ok(host_reference_core::char_offset_view(text, &id, *start, *len))
            }
            _ => Ok(Tier1 {
                markdown: text.to_string(),
                source_map: SourceMap { spans: vec![Span { source: id, origin: 0..text.len() }] },
            }),
        }
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
        push_inline(c, &mut s);
    }
    s
}

/// Append one inline element's textual content, recursing into markup so emphasis, strong, literal,
/// reference and the rest contribute their text rather than being silently dropped.
fn push_inline(element: &TextOrInlineElement, out: &mut String) {
    use TextOrInlineElement as T;
    match element {
        T::String(t) => out.push_str(t.as_str()),
        // Markup wrapping further inline elements: recurse into the children.
        T::Emphasis(e) => out.push_str(&inline_text(e.children())),
        T::Strong(e) => out.push_str(&inline_text(e.children())),
        T::Reference(e) => out.push_str(&inline_text(e.children())),
        T::FootnoteReference(e) => out.push_str(&inline_text(e.children())),
        T::CitationReference(e) => out.push_str(&inline_text(e.children())),
        T::SubstitutionReference(e) => out.push_str(&inline_text(e.children())),
        T::TitleReference(e) => out.push_str(&inline_text(e.children())),
        T::Abbreviation(e) => out.push_str(&inline_text(e.children())),
        T::Acronym(e) => out.push_str(&inline_text(e.children())),
        T::Superscript(e) => out.push_str(&inline_text(e.children())),
        T::Subscript(e) => out.push_str(&inline_text(e.children())),
        T::Inline(e) => out.push_str(&inline_text(e.children())),
        T::Problematic(e) => out.push_str(&inline_text(e.children())),
        T::Generated(e) => out.push_str(&inline_text(e.children())),
        // Markup wrapping plain strings.
        T::Literal(e) => e.children().iter().for_each(|t| out.push_str(t)),
        T::Math(e) => e.children().iter().for_each(|t| out.push_str(t)),
        T::TargetInline(e) => e.children().iter().for_each(|t| out.push_str(t)),
        T::RawInline(e) => e.children().iter().for_each(|t| out.push_str(t)),
        // An inline image carries no textual content.
        T::ImageInline(_) => {}
    }
}
