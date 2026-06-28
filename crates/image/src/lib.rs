//! The raster-image normaliser: deterministic, attested metadata only (call/0032). It reports the
//! format and pixel dimensions from the header, plus any embedded EXIF tags. It does NOT recognise
//! pixels: OCR and caption recognition are non-deterministic and ride the overlay adapter (call/0030),
//! not this attested reader. The source map is whole-document.

use std::io::Cursor;

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct ImageNormalizer;

impl Normalizer for ImageNormalizer {
    fn modality(&self) -> Modality {
        Modality::Raster
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::None, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(
            source.hint,
            Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "webp")
        )
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = shape(source.bytes)?;
        Ok(Tier0 {
            raw_tokens: source.bytes.len(),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }

    fn view(&self, source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
        let id = content_id(source.bytes);
        Ok(Tier1 {
            markdown: shape(source.bytes)?,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn shape(bytes: &[u8]) -> Result<String, Error> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| Error::Parse(format!("image: {e}")))?;
    let format = reader
        .format()
        .map(|f| format!("{f:?}").to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());
    let (w, h) =
        reader.into_dimensions().map_err(|e| Error::Parse(format!("image: {e}")))?;
    let mut out = format!("image: {format} {w}x{h}\n");
    if let Some(exif) = read_exif(bytes) {
        out.push_str("exif:\n");
        out.push_str(&exif);
    }
    Ok(out)
}

fn read_exif(bytes: &[u8]) -> Option<String> {
    let mut cursor = Cursor::new(bytes);
    let exif = exif::Reader::new().read_from_container(&mut cursor).ok()?;
    let mut fields: Vec<(String, String)> = exif
        .fields()
        .map(|f| (f.tag.to_string(), f.display_value().to_string()))
        .collect();
    if fields.is_empty() {
        return None;
    }
    fields.sort();
    let mut out = String::new();
    for (tag, value) in fields {
        out.push_str(&format!("- {tag}: {value}\n"));
    }
    Some(out)
}
