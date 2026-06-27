//! The RTF normaliser. RTF carries no heading structure, so the skeleton is the de-styled text: a
//! paragraph and character count with a short preview. The view returns the full de-styled text
//! rather than the raw RTF markup, since the markup is not what a reader wants. The source map is
//! whole-document for now.

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};
use rtf_parser::RtfDocument;

pub struct RtfNormalizer;

impl RtfNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for RtfNormalizer {
    fn modality(&self) -> Modality {
        Modality::Prose
    }

    fn capabilities(&self) -> Caps {
        // De-styled text only: no structural roles, no round-trip (styling is dropped), no
        // write-back, no recognition.
        Caps { round_trip: false, write_back: false, semantic: Semantic::None, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("rtf"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let plain = plain_text(text)?;
        let outline = rtf_shape(text, &plain);
        Ok(Tier0 {
            raw_tokens: count_tokens(text),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }

    fn view(&self, source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        Ok(Tier1 {
            markdown: plain_text(text)?,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn plain_text(text: &str) -> Result<String, Error> {
    let doc = RtfDocument::try_from(text).map_err(|e| Error::Parse(format!("rtf: {e}")))?;
    Ok(doc.get_text())
}

fn rtf_shape(raw: &str, plain: &str) -> String {
    let chars = plain.chars().count();
    // get_text concatenates the runs without the paragraph breaks, so the breaks are counted from
    // the raw `\par` control words instead.
    let paragraphs = if plain.trim().is_empty() { 0 } else { count_paragraphs(raw).max(1) };
    let preview: String = plain.chars().take(100).collect();
    let mut out = format!("rtf: {paragraphs} paragraphs, {chars} characters\n");
    if !preview.trim().is_empty() {
        out.push_str(&format!("preview: {}\n", preview.trim()));
    }
    out
}

/// Count the `\par` control words in the raw RTF, the paragraph breaks. A following alphanumeric
/// character means a longer control word (such as `\pard`), so those are not counted.
fn count_paragraphs(rtf: &str) -> usize {
    let bytes = rtf.as_bytes();
    let mut count = 0;
    let mut search = 0;
    while let Some(rel) = rtf[search..].find("\\par") {
        let after = search + rel + 4;
        if !bytes.get(after).is_some_and(|b| b.is_ascii_alphanumeric()) {
            count += 1;
        }
        search = after;
    }
    count
}
