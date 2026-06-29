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

use tempfile::Builder;

use host_reference_core::{Caps, Error, Modality, Normalizer, Semantic, Source, Tier0};

pub struct OcrNormalizer;

impl Normalizer for OcrNormalizer {
    fn modality(&self) -> Modality {
        Modality::Raster
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::None, ocr: true }
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "webp"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let outline = render(&recognise(source)?, &helper_version()?);
        Ok(host_reference_core::Tier0::whole(source.bytes, outline))
    }
}

fn render(text: &str, engine: &str) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut out = format!(
        "ocr: {} line{} (engine: {engine})\n",
        lines.len(),
        if lines.len() == 1 { "" } else { "s" }
    );
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

/// The helper's reported version, recorded into the attested skeleton. The recognition is a pure
/// function of the source bytes and this engine version; out-of-process the engine cannot be a
/// Cargo.lock dependency, so the attestation declares which engine produced it, and a different
/// helper version yields a visibly different attested output rather than a silent divergence
/// (finding 9, call/0034).
fn helper_version() -> Result<String, Error> {
    let output = Command::new(helper_path())
        .arg("--version")
        .output()
        .map_err(|e| Error::Parse(format!("ocr: cannot run helper: {e}")))?;
    if !output.status.success() {
        return Err(Error::Parse("ocr: helper does not report a version".into()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn recognise(source: &Source) -> Result<String, Error> {
    let ext = source.hint.unwrap_or("png");
    // An O_EXCL temp file with a random name, removed on drop: no predictable shared path a symlink
    // could pre-empt, and no collision between two concurrent calls on the same bytes (finding 10).
    let mut file = Builder::new()
        .prefix("host-reference-ocr-")
        .suffix(&format!(".{ext}"))
        .tempfile()
        .map_err(|e| Error::Parse(format!("ocr: staging image: {e}")))?;
    file.write_all(source.bytes).map_err(|e| Error::Parse(format!("ocr: staging image: {e}")))?;

    // `file` is held open across the helper run (the helper reads it by path) and removed on drop.
    let output = Command::new(helper_path())
        .arg(file.path())
        .output()
        .map_err(|e| Error::Parse(format!("ocr: cannot run helper: {e}")))?;
    if !output.status.success() {
        return Err(Error::Parse(format!(
            "ocr: helper failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(|e| Error::Parse(format!("ocr: {e}")))
}
