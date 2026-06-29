//! The BibTeX normaliser: a bibliography is a set of entries, so the skeleton is the entry list, each
//! a citation key with its entry type, sorted by key. The full content round-trips through the view.
//! The source map is whole-document for now.

use biblatex::Bibliography;
use host_reference_core::{
    content_id, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span, SpanSelector,
    Tier0, Tier1,
};

pub struct BibtexNormalizer;

impl BibtexNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for BibtexNormalizer {
    fn modality(&self) -> Modality {
        Modality::StructuredData
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: true, write_back: false, semantic: Semantic::Full, ocr: false }
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["bib", "bibtex"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let outline = bibtex_shape(text)?;
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

fn bibtex_shape(text: &str) -> Result<String, Error> {
    let bib = Bibliography::parse(text).map_err(|e| Error::Parse(format!("bibtex: {e}")))?;
    let mut entries: Vec<(String, String)> =
        bib.iter().map(|e| (e.key.clone(), format!("{:?}", e.entry_type))).collect();
    entries.sort();
    let mut out = format!("bibliography: {} entries\n", entries.len());
    for (key, kind) in entries {
        out.push_str(&format!("- {key} [{kind}]\n"));
    }
    Ok(out)
}
