//! The OCR normaliser, the first out-of-process plugin (call/0033). It carries no OCR engine and no
//! model. It writes the image to a temporary file, runs the `host-reference-ocr-helper` binary as a
//! separate process, and reads the recognised text from its stdout. The licence-encumbered engine and
//! the CC-BY-SA-4.0 ocrs models stay behind that arms-length boundary, so this crate and its
//! dependents are permissive, an aggregation with the helper rather than a derivative of it. The
//! helper must be installed; its path comes from HOST_REFERENCE_OCR_HELPER, else the binary name on
//! PATH. Recognition is an attested parse of a fixed engine over fixed bytes, so the skeleton is the
//! recognised text. The source map is whole-document.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct OcrNormalizer;

impl Normalizer for OcrNormalizer {
    fn modality(&self) -> Modality {
        Modality::Raster
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::None, ocr: true }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(
            source.hint,
            Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "webp")
        )
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = render(&recognise(source)?);
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
            markdown: render(&recognise(source)?),
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn render(text: &str) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut out = format!("ocr: {} line{}\n", lines.len(), if lines.len() == 1 { "" } else { "s" });
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn helper_path() -> PathBuf {
    std::env::var_os("HOST_REFERENCE_OCR_HELPER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("host-reference-ocr-helper"))
}

fn recognise(source: &Source) -> Result<String, Error> {
    let ext = source.hint.unwrap_or("png");
    let path =
        std::env::temp_dir().join(format!("host-reference-ocr-{}.{ext}", content_id(source.bytes)));
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(source.bytes))
        .map_err(|e| Error::Parse(format!("ocr: staging image: {e}")))?;

    let result = Command::new(helper_path()).arg(&path).output();
    let _ = std::fs::remove_file(&path);

    let output = result.map_err(|e| Error::Parse(format!("ocr: cannot run helper: {e}")))?;
    if !output.status.success() {
        return Err(Error::Parse(format!(
            "ocr: helper failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(|e| Error::Parse(format!("ocr: {e}")))
}
