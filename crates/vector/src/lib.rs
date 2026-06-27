//! The vector normaliser: SVG. Raw path data is verbose and not interpretable, so the skeleton is
//! a summary: the dimensions, the element counts by tag, and the human-readable labels (`text`,
//! `title`, `desc`). The summary is lossy, so it does not round-trip and offers no write-back; the
//! source map is whole-document.

use std::collections::BTreeMap;

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct SvgNormalizer;

fn floor_boundary(text: &str, mut i: usize) -> usize {
    if i > text.len() {
        i = text.len();
    }
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

impl SvgNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for SvgNormalizer {
    fn modality(&self) -> Modality {
        Modality::Vector
    }

    fn capabilities(&self) -> Caps {
        // The summary drops the geometry, so it is lossy: no round-trip and no write-back; labels
        // and the element tally give partial semantics.
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("svg"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let doc = roxmltree::Document::parse(text).map_err(|e| Error::Parse(format!("svg: {e}")))?;
        let root = doc.root_element();

        let w = root.attribute("width").unwrap_or("?");
        let h = root.attribute("height").unwrap_or("?");
        let view_box = root.attribute("viewBox");

        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut labels: Vec<String> = Vec::new();
        for node in root.descendants().filter(|n| n.is_element()) {
            let name = node.tag_name().name();
            *counts.entry(name).or_insert(0) += 1;
            if matches!(name, "text" | "title" | "desc") {
                let label = node
                    .text()
                    .unwrap_or("")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if !label.is_empty() {
                    labels.push(label);
                }
            }
        }

        let mut out = format!("- svg: {w} x {h}");
        if let Some(vb) = view_box {
            out.push_str(&format!(" (viewBox {vb})"));
        }
        out.push('\n');
        out.push_str("- elements:\n");
        for (tag, n) in &counts {
            out.push_str(&format!("  - {tag}: {n}\n"));
        }
        if !labels.is_empty() {
            out.push_str("- labels:\n");
            for label in &labels {
                out.push_str(&format!("  - {label}\n"));
            }
        }

        Ok(Tier0 {
            raw_tokens: count_tokens(text),
            normalised_tokens: count_tokens(&out),
            markdown: out,
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
                let s = floor_boundary(text, *start);
                (s, floor_boundary(text, s + *len))
            }
            _ => (0, text.len()),
        };
        Ok(Tier1 {
            markdown: text[start..end].to_string(),
            source_map: SourceMap { spans: vec![Span { source: id, origin: start..end }] },
        })
    }
}
