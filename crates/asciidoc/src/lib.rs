//! The AsciiDoc normaliser: the skeleton is the section heading outline, walked from asciidork's
//! parsed document. The full text round-trips through the view. The source map is whole-document for
//! now.

use asciidork_ast::{BlockContent, DocContent, Section};
use asciidork_parser::prelude::*;
use bumpalo::Bump;
use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct AsciidocNormalizer;

impl AsciidocNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for AsciidocNormalizer {
    fn modality(&self) -> Modality {
        Modality::Prose
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: true, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("adoc" | "asciidoc" | "asc"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let outline = asciidoc_shape(text)?;
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

fn asciidoc_shape(text: &str) -> Result<String, Error> {
    let bump = Bump::new();
    let cwd = std::path::PathBuf::from(".").into();
    let parser = Parser::from_str(text, SourceFile::Stdin { cwd }, &bump);
    let result =
        parser.parse().map_err(|d| Error::Parse(format!("asciidoc: {} diagnostics", d.len())))?;
    let mut out = String::new();
    if let DocContent::Sections(sectioned) = &result.document.content {
        for section in &sectioned.sections {
            emit_section(section, &mut out);
        }
    }
    if out.is_empty() {
        out.push_str("(no sections)\n");
    }
    Ok(out)
}

fn emit_section(section: &Section, out: &mut String) {
    let title = section.heading.plain_text().concat();
    let indent = "  ".repeat(section.level.saturating_sub(1) as usize);
    out.push_str(&format!("{indent}- {title}\n"));
    for block in &section.blocks {
        if let BlockContent::Section(sub) = &block.content {
            emit_section(sub, out);
        }
    }
}
