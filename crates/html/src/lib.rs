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
    // Track fenced code blocks: a line whose trimmed form opens with three or more backticks
    // toggles the fence, and a `#` line inside one is code, not a heading.
    let mut in_fence = false;
    for line in md.lines() {
        let t = line.trim_start();
        if t.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
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
                spans: vec![Span {
                    source: content_id(source.bytes),
                    origin: 0..source.bytes.len(),
                }],
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
            // The HTML source map is in converted-markdown space: htmd rewrites positions, so the
            // raw-HTML origin is not recoverable without a position map htmd does not expose. The span
            // therefore reports the window within the converted markdown that was returned.
            source_map: SourceMap { spans: vec![Span { source: id, origin: start..end }] },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_origin_is_the_returned_window_not_the_raw_length() {
        // The raw HTML is far longer than the markdown htmd produces from it.
        let html = b"<main><h1>Title</h1><p>Body paragraph with several words.</p></main>";
        let source = Source { bytes: html, hint: Some("html") };
        let v = HtmlNormalizer
            .view(&source, &SpanSelector::CharOffset { start: 0, len: 5 })
            .expect("view");
        let span = &v.source_map.spans[0];
        // The reported origin spans exactly the returned window, measured in converted-markdown
        // bytes, not the raw-HTML byte length.
        assert_eq!(span.origin.end - span.origin.start, v.markdown.len());
        assert_ne!(span.origin.end, html.len());
    }
}
