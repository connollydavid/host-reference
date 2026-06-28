//! The config normaliser: INI, Java properties, and dotenv files. The skeleton is the key structure
//! (sections and keys for INI, the sorted key set for properties and dotenv); the values are config
//! payload that the windowed view returns in full. The dotenv reader emits keys only and does not
//! interpolate a value against the process environment, so the attested layer stays host independent.
//! The source map is whole-document for now.

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct ConfigNormalizer;

impl ConfigNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for ConfigNormalizer {
    fn modality(&self) -> Modality {
        Modality::StructuredData
    }

    fn capabilities(&self) -> Caps {
        // The key structure is captured; the full content round-trips through the view; no edit
        // write-back and no recognition.
        Caps { round_trip: true, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("ini" | "properties" | "env"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let outline = match source.hint {
            Some("properties") => properties_shape(text)?,
            Some("env") => env_shape(text)?,
            _ => ini_shape(text)?,
        };
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
                host_reference_core::char_offset_window(text, *start, *len)
            }
            _ => (0, text.len()),
        };
        Ok(Tier1 {
            markdown: text[start..end].to_string(),
            source_map: SourceMap { spans: vec![Span { source: id, origin: start..end }] },
        })
    }
}

/// INI: each section (the keyless preamble shown as `(default)`) with its keys, sorted.
fn ini_shape(text: &str) -> Result<String, Error> {
    let ini = ini::Ini::load_from_str(text).map_err(|e| Error::Parse(format!("ini: {e}")))?;
    let mut out = String::new();
    for (section, props) in ini.iter() {
        out.push_str(&format!("[{}]\n", section.unwrap_or("(default)")));
        let mut keys: Vec<&str> = props.iter().map(|(k, _)| k).collect();
        keys.sort_unstable();
        keys.dedup();
        for k in keys {
            out.push_str(&format!("- {k}\n"));
        }
    }
    Ok(out)
}

/// Java properties: the sorted key set.
fn properties_shape(text: &str) -> Result<String, Error> {
    let map = java_properties::read(text.as_bytes())
        .map_err(|e| Error::Parse(format!("properties: {e}")))?;
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    let mut out = format!("properties: {} keys\n", keys.len());
    for k in keys {
        out.push_str(&format!("- {k}\n"));
    }
    Ok(out)
}

/// dotenv: the sorted key set. Only the keys are read, so the output does not depend on any value
/// interpolation against the process environment.
fn env_shape(text: &str) -> Result<String, Error> {
    let mut keys = Vec::new();
    for item in dotenvy::Iter::new(text.as_bytes()) {
        let (k, _v) = item.map_err(|e| Error::Parse(format!("env: {e}")))?;
        keys.push(k);
    }
    keys.sort();
    keys.dedup();
    let mut out = format!("env: {} keys\n", keys.len());
    for k in &keys {
        out.push_str(&format!("- {k}\n"));
    }
    Ok(out)
}
