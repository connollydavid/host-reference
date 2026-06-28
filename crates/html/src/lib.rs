//! The HTML normaliser: HTML5 to markdown. htmd parses real HTML5 leniently and converts it to
//! markdown (the token-optimal target for web content); the skeleton is that markdown's heading
//! outline. The conversion drops chrome and exact markup, so it does not round-trip and offers no
//! write-back; the source map is whole-document.

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct HtmlNormalizer;

impl HtmlNormalizer {
    fn markdown(&self, source: &Source) -> Result<String, Error> {
        let text = std::str::from_utf8(source.bytes)
            .map_err(|e| Error::Parse(format!("not UTF-8: {e}")))?;
        // De-chrome: drop script and style, and the structural chrome (nav, header, footer,
        // aside), so the markdown is content rather than navigation and trackers.
        htmd::HtmlToMarkdown::builder()
            .skip_tags(vec!["script", "style", "nav", "header", "footer", "aside", "noscript"])
            .build()
            .convert(text)
            .map_err(|e| Error::Parse(format!("html: {e}")))
    }
}

/// The heading outline of markdown: each ATX heading as an indented bullet.
fn heading_outline(md: &str) -> String {
    let mut out = String::new();
    for line in md.lines() {
        let t = line.trim_start();
        let hashes = t.bytes().take_while(|b| *b == b'#').count();
        if hashes >= 1 && t.as_bytes().get(hashes) == Some(&b' ') {
            out.push_str(&"  ".repeat(hashes - 1));
            out.push_str("- ");
            out.push_str(t[hashes + 1..].trim());
            out.push('\n');
        }
    }
    if out.is_empty() {
        out.push_str("- (no headings)\n");
    }
    out
}

impl Normalizer for HtmlNormalizer {
    fn modality(&self) -> Modality {
        Modality::Prose
    }

    fn capabilities(&self) -> Caps {
        // html to markdown drops chrome and exact markup, so it is lossy: no round-trip and no
        // write-back; headings give partial structure.
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("html" | "htm" | "xhtml"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let raw = std::str::from_utf8(source.bytes)
            .map_err(|e| Error::Parse(format!("not UTF-8: {e}")))?;
        let md = self.markdown(source)?;
        let outline = heading_outline(&md);
        Ok(Tier0 {
            raw_tokens: count_tokens(raw),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap {
                spans: vec![Span { source: content_id(source.bytes), origin: 0..source.bytes.len() }],
            },
        })
    }

    fn view(&self, source: &Source, select: &SpanSelector) -> Result<Tier1, Error> {
        let md = self.markdown(source)?;
        let id = content_id(source.bytes);
        let (start, end) = match select {
            SpanSelector::CharOffset { start, len } => {
                host_reference_core::char_offset_window(&md, *start, *len)
            }
            _ => (0, md.len()),
        };
        Ok(Tier1 {
            markdown: md[start..end].to_string(),
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}
