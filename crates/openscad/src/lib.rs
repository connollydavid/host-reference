//! The OpenSCAD normaliser, an out-of-process plugin (call/0033, call/0034), the same shape as the OCR
//! plugin. It carries no parser and no model. It writes the `.scad` to a temporary file, runs the
//! `host-reference-openscad-helper` binary as a separate process, and reads back the kind of each
//! top-level statement, one per line. The GPL openscad-rs parser stays behind that arms-length
//! boundary, so this crate and its dependents are permissive, an aggregation with the helper rather
//! than a derivative. The skeleton is a deterministic tally of the statement kinds (call/0032). The
//! helper must be installed; its path comes from HOST_REFERENCE_OPENSCAD_HELPER, else the binary name
//! on PATH. The source map is whole-document.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use tempfile::Builder;

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct OpenscadNormalizer;

impl Normalizer for OpenscadNormalizer {
    fn modality(&self) -> Modality {
        Modality::EngineeringGeometry
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("scad"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = render(&run_helper(source)?, &helper_version()?);
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
        Ok(Tier1 {
            markdown: render(&run_helper(source)?, &helper_version()?),
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn render(kinds: &str, engine: &str) -> String {
    let mut tally: Vec<(String, usize)> = Vec::new();
    let mut total = 0usize;
    for kind in kinds.lines().map(str::trim).filter(|l| !l.is_empty()) {
        total += 1;
        match tally.iter_mut().find(|(k, _)| k == kind) {
            Some(t) => t.1 += 1,
            None => tally.push((kind.to_string(), 1)),
        }
    }
    tally.sort();
    let mut out = format!(
        "openscad: {total} statement{} (engine: {engine})\n",
        if total == 1 { "" } else { "s" }
    );
    for (kind, count) in tally {
        if count > 1 {
            out.push_str(&format!("- {kind} (x{count})\n"));
        } else {
            out.push_str(&format!("- {kind}\n"));
        }
    }
    out
}

fn helper_path() -> PathBuf {
    std::env::var_os("HOST_REFERENCE_OPENSCAD_HELPER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("host-reference-openscad-helper"))
}

/// The helper's reported version, recorded into the attested skeleton, so the structure tally is a
/// pure function of the source bytes and this engine version; a different helper version yields a
/// visibly different attested output rather than a silent divergence (finding 9, call/0034).
fn helper_version() -> Result<String, Error> {
    let output = Command::new(helper_path())
        .arg("--version")
        .output()
        .map_err(|e| Error::Parse(format!("openscad: cannot run helper: {e}")))?;
    if !output.status.success() {
        return Err(Error::Parse("openscad: helper does not report a version".into()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_helper(source: &Source) -> Result<String, Error> {
    // An O_EXCL temp file with a random name, removed on drop: no predictable shared path a symlink
    // could pre-empt, and no collision between two concurrent calls on the same bytes (finding 10).
    let mut file = Builder::new()
        .prefix("host-reference-openscad-")
        .suffix(".scad")
        .tempfile()
        .map_err(|e| Error::Parse(format!("openscad: staging source: {e}")))?;
    file.write_all(source.bytes)
        .map_err(|e| Error::Parse(format!("openscad: staging source: {e}")))?;

    // `file` is held open across the helper run (the helper reads it by path) and removed on drop.
    let output = Command::new(helper_path())
        .arg(file.path())
        .output()
        .map_err(|e| Error::Parse(format!("openscad: cannot run helper: {e}")))?;
    if !output.status.success() {
        return Err(Error::Parse(format!(
            "openscad: helper failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(|e| Error::Parse(format!("openscad: {e}")))
}
