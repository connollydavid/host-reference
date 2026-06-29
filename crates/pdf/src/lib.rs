//! The fixed-layout normaliser: born-digital PDF. lopdf reads the page count and the document title;
//! pdf-extract pulls the text for a preview, and the view returns the full extracted text. The source
//! map is whole-document for now. Scanned PDFs (image-only) belong to the recognition path, not here.

use host_reference_core::{
    content_id, guard_panic, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
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

    fn extensions(&self) -> &'static [&'static str] {
        &["pdf"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let outline = pdf_shape(source.bytes)?;
        Ok(host_reference_core::Tier0::whole(source.bytes, outline))
    }

    fn view(&self, source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
        let id = content_id(source.bytes);
        // pdf-extract carries ~20 panic!/todo! sites for structures lopdf loads but it cannot
        // handle; the guard turns that unwind into an explicit refusal instead of a process abort
        // (finding 3, call/0031). The view's whole payload is the extracted text, so a panic refuses.
        let text = guard_panic("pdf", || {
            pdf_extract::extract_text_from_mem(source.bytes)
                .map_err(|e| Error::Parse(format!("pdf: {e}")))
        })?;
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
    // The skeleton's load-bearing facts (title, page count) come from lopdf, which already parsed
    // the document. The preview is best-effort: if pdf-extract panics on a structure it cannot
    // handle, the guard degrades to an empty preview rather than aborting (finding 3).
    let text = guard_panic("pdf preview", || {
        Ok(pdf_extract::extract_text_from_mem(bytes).unwrap_or_default())
    })
    .unwrap_or_default();
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
    Some(decode_pdf_text(title))
}

/// Decode a PDF text string. A PDF string is UTF-16BE when it opens with the byte-order mark
/// 0xFE 0xFF (a Word- or Acrobat-exported title is commonly UTF-16BE); otherwise it is
/// PDFDocEncoding, approximated by a lossy UTF-8 read since ASCII is the common subset.
fn decode_pdf_text(bytes: &[u8]) -> String {
    match bytes.strip_prefix(&[0xFE, 0xFF]) {
        Some(rest) => {
            let units: Vec<u16> = rest
                .chunks(2)
                .map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]))
                .collect();
            String::from_utf16_lossy(&units)
        }
        None => String::from_utf8_lossy(bytes).to_string(),
    }
}
