//! The Org-mode normaliser: the skeleton is the headline outline, each headline indented by its
//! level. The full text round-trips through the view. The source map is whole-document for now.

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};
use orgize::{Element, Event, Org};

pub struct OrgNormalizer;

impl OrgNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for OrgNormalizer {
    fn modality(&self) -> Modality {
        Modality::Prose
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: true, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("org"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let outline = org_shape(text);
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

fn floor_boundary(text: &str, mut i: usize) -> usize {
    if i > text.len() {
        i = text.len();
    }
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn org_shape(text: &str) -> String {
    let org = Org::parse(text);
    let mut out = String::new();
    for event in org.iter() {
        if let Event::Start(Element::Title(title)) = event {
            let indent = "  ".repeat(title.level.saturating_sub(1));
            out.push_str(&format!("{indent}- {}\n", title.raw.trim()));
        }
    }
    if out.is_empty() {
        out.push_str("(no headlines)\n");
    }
    out
}
