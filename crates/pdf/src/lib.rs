//! The fixed-layout normaliser: born-digital PDF. lopdf reads the page count and the document title;
//! pdf-extract pulls the text for a preview, and the view returns the full extracted text. The source
//! map is whole-document for now. Scanned PDFs (image-only) belong to the recognition path, not here.

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};
use lopdf::{Document, Object};

pub struct PdfNormalizer;

impl Normalizer for PdfNormalizer {
    fn modality(&self) -> Modality {
        Modality::FixedLayout
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("pdf"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = pdf_shape(source.bytes)?;
        let lossy = String::from_utf8_lossy(source.bytes);
        Ok(Tier0 {
            raw_tokens: count_tokens(&lossy),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }

    fn view(&self, source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
        let id = content_id(source.bytes);
        let text = pdf_extract::extract_text_from_mem(source.bytes)
            .map_err(|e| Error::Parse(format!("pdf: {e}")))?;
        Ok(Tier1 {
            markdown: text,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn pdf_shape(bytes: &[u8]) -> Result<String, Error> {
    let doc = Document::load_mem(bytes).map_err(|e| Error::Parse(format!("pdf: {e}")))?;
    let mut out = String::new();
    if let Some(title) = pdf_title(&doc) {
        out.push_str(&format!("title: {title}\n"));
    }
    out.push_str(&format!("pages: {}\n", doc.get_pages().len()));
    let text = pdf_extract::extract_text_from_mem(bytes).unwrap_or_default();
    let preview: String =
        text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(120).collect();
    if !preview.is_empty() {
        out.push_str(&format!("preview: {preview}\n"));
    }
    Ok(out)
}

fn pdf_title(doc: &Document) -> Option<String> {
    let info_id = match doc.trailer.get(b"Info") {
        Ok(Object::Reference(id)) => *id,
        _ => return None,
    };
    let dict = doc.get_object(info_id).ok()?.as_dict().ok()?;
    let title = dict.get(b"Title").ok()?.as_str().ok()?;
    Some(String::from_utf8_lossy(title).to_string())
}
