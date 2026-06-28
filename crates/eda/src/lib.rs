//! The engineering-EDA normaliser: a deterministic structure-and-metadata summary per format
//! (call/0032). KiCad schematics and boards are S-expressions, summarised by a tally of their
//! top-level forms; Eagle schematics and boards are XML, summarised by an element tally; Gerber
//! (RS-274X) is summarised by its command count. The source map is whole-document for now.

use std::io::{BufReader, Cursor};

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct EdaNormalizer;

impl Normalizer for EdaNormalizer {
    fn modality(&self) -> Modality {
        Modality::EngineeringEda
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(
            source.hint,
            Some("kicad_sch" | "kicad_pcb" | "kicad" | "gbr" | "gerber" | "eagle" | "brd" | "sch")
        )
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = shape(source)?;
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
            markdown: shape(source)?,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn shape(source: &Source) -> Result<String, Error> {
    match source.hint {
        Some("gbr" | "gerber") => gerber_shape(source.bytes),
        Some("eagle" | "brd" | "sch") => eagle_shape(source.bytes),
        _ => kicad_shape(source.bytes),
    }
}

fn kicad_shape(bytes: &[u8]) -> Result<String, Error> {
    let text = std::str::from_utf8(bytes).map_err(|e| Error::Parse(format!("kicad: {e}")))?;
    let value = lexpr::from_str(text).map_err(|e| Error::Parse(format!("kicad: {e}")))?;
    let mut tally: Vec<(String, usize)> = Vec::new();
    if let Some(iter) = value.list_iter() {
        for elem in iter {
            if let Some(mut inner) = elem.list_iter() {
                if let Some(head) = inner.next().and_then(|h| h.as_symbol()) {
                    bump(&mut tally, head);
                }
            }
        }
    }
    Ok(render_tally("kicad", "forms", tally))
}

fn eagle_shape(bytes: &[u8]) -> Result<String, Error> {
    let text = std::str::from_utf8(bytes).map_err(|e| Error::Parse(format!("eagle: {e}")))?;
    let doc = roxmltree::Document::parse(text).map_err(|e| Error::Parse(format!("eagle: {e}")))?;
    let mut tally: Vec<(String, usize)> = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        bump(&mut tally, node.tag_name().name());
    }
    Ok(render_tally("eagle", "elements", tally))
}

fn gerber_shape(bytes: &[u8]) -> Result<String, Error> {
    let reader = BufReader::new(Cursor::new(bytes));
    let doc =
        gerber_parser::parse(reader).map_err(|(_, e)| Error::Parse(format!("gerber: {e:?}")))?;
    Ok(format!("gerber: {} commands\n", doc.commands.len()))
}

fn bump(tally: &mut Vec<(String, usize)>, key: &str) {
    match tally.iter_mut().find(|(k, _)| k == key) {
        Some(t) => t.1 += 1,
        None => tally.push((key.to_string(), 1)),
    }
}

fn render_tally(label: &str, unit: &str, mut tally: Vec<(String, usize)>) -> String {
    let total: usize = tally.iter().map(|(_, c)| c).sum();
    tally.sort();
    let mut out = format!("{label}: {total} {unit}\n");
    for (kind, count) in tally {
        if count > 1 {
            out.push_str(&format!("- {kind} (x{count})\n"));
        } else {
            out.push_str(&format!("- {kind}\n"));
        }
    }
    out
}
