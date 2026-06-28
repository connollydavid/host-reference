//! The prose normaliser: markdown and plain text. It preserves the UTF-8 content, so the full
//! view round-trips byte for byte, and it builds a heading-outline skeleton with a per-heading
//! source map. Byte ranges fall on UTF-8 boundaries, so a multibyte script (for example Standard
//! Chinese) is handled correctly.

use host_reference_core::{
    content_id, count_tokens, Caps, Edit, Error, Modality, Normalizer, Patch, Semantic, Source,
    SourceMap, Span, SpanSelector, Tier0, Tier1,
};

pub struct ProseNormalizer;

impl ProseNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

/// The hash count of an ATX heading line (`#`, `##`, ...), or zero when the line is not a heading.
fn heading_level(line: &str) -> usize {
    let t = line.trim_start();
    let hashes = t.bytes().take_while(|b| *b == b'#').count();
    if hashes >= 1 && t.as_bytes().get(hashes) == Some(&b' ') {
        hashes
    } else {
        0
    }
}

fn heading_title(line: &str) -> &str {
    let t = line.trim_start();
    let hashes = t.bytes().take_while(|b| *b == b'#').count();
    t[hashes + 1..].trim()
}

/// The byte range of the section a heading opens: from the heading to the next heading at the same
/// or a higher level, or to the end of the document.
fn section_range(text: &str, title: &str) -> Option<(usize, usize)> {
    let mut start = None;
    let mut level = 0usize;
    let mut offset = 0usize;
    let mut in_fence = false;
    for line in text.split_inclusive('\n') {
        // A fenced code block toggles on a ``` line; a `#` line inside it is not a heading, the
        // same fence awareness the skeleton outline applies (finding 8).
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            offset += line.len();
            continue;
        }
        let h = if in_fence { 0 } else { heading_level(line) };
        if let Some(s) = start {
            if h >= 1 && h <= level {
                return Some((s, offset));
            }
        } else if h >= 1 && heading_title(line) == title {
            start = Some(offset);
            level = h;
        }
        offset += line.len();
    }
    start.map(|s| (s, text.len()))
}

impl Normalizer for ProseNormalizer {
    fn modality(&self) -> Modality {
        Modality::Prose
    }

    fn capabilities(&self) -> Caps {
        // The full content round-trips for UTF-8 input; the outline is editable; headings are
        // partial semantic structure; no recognition is involved.
        Caps { round_trip: true, write_back: true, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("md" | "markdown" | "txt" | "text"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);

        let mut outline = String::new();
        let mut spans = Vec::new();
        let mut offset = 0usize;
        // A line whose trimmed form opens with three or more backticks toggles a fenced code block;
        // a `#` line inside a fence is code, not a heading, so the scan ignores it.
        let mut in_fence = false;
        for line in text.split_inclusive('\n') {
            if line.trim_start().starts_with("```") {
                in_fence = !in_fence;
            } else if !in_fence {
                let level = heading_level(line);
                if level >= 1 {
                    outline.push_str(&"  ".repeat(level - 1));
                    outline.push_str("- ");
                    outline.push_str(heading_title(line));
                    outline.push('\n');
                    spans.push(Span { source: id.clone(), origin: offset..offset + line.len() });
                }
            }
            offset += line.len();
        }

        // A document with no headings falls back to a note and a whole-document span.
        if spans.is_empty() {
            outline.push_str("- (no headings)\n");
            spans.push(Span { source: id.clone(), origin: 0..source.bytes.len() });
        }

        Ok(Tier0 {
            raw_tokens: count_tokens(text),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap { spans },
        })
    }

    fn view(&self, source: &Source, select: &SpanSelector) -> Result<Tier1, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let (start, end) = match select {
            SpanSelector::Section(title) => section_range(text, title)
                .ok_or_else(|| Error::Parse(format!("no section titled {title:?}")))?,
            SpanSelector::CharOffset { start, len } => {
                host_reference_core::char_offset_window(text, *start, *len)
            }
            _ => return Err(Error::Unsupported("view selector")),
        };
        Ok(Tier1 {
            markdown: text[start..end].to_string(),
            source_map: SourceMap { spans: vec![Span { source: id, origin: start..end }] },
        })
    }

    fn put(&self, source: &Source, edit: &Edit) -> Result<Patch, Error> {
        // A well-behaved lens for UTF-8 text: replace the edited span and re-emit the bytes.
        let text = self.text(source)?;
        let start = host_reference_core::floor_boundary(text, edit.at.origin.start);
        let end = host_reference_core::floor_boundary(text, edit.at.origin.end.max(start));
        let mut out = String::with_capacity(text.len());
        out.push_str(&text[..start]);
        out.push_str(&edit.replacement);
        out.push_str(&text[end..]);
        Ok(Patch { bytes: out.into_bytes() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_range_skips_a_heading_inside_a_fence() {
        // A `#` line inside a fenced code block must not be mistaken for a section boundary
        // (finding 8, the view-Section sibling of the outline fix).
        let text = "# Real\nintro\n```\n# not a heading\n```\nbody\n# Next\ntail\n";
        let (start, end) = section_range(text, "Real").expect("the real section resolves");
        // The section runs to the next real heading (# Next), spanning the whole fenced block.
        assert!(text[start..end].contains("# not a heading"), "the fenced line stays inside the section");
        assert!(!text[start..end].contains("# Next"), "it stops at the next real heading");
    }
}
