//! The structured-data normaliser: JSON, TOML, YAML, CSV, TSV, generic XML (RSS, Atom, plain XML),
//! JSON Lines, and the Jupyter notebook. JSON and TOML and YAML parse to a value whose shape (keys
//! and types, sorted) is the skeleton; CSV and TSV give the columns and row count; XML walks the
//! element tree,
//! collapsing repeated siblings into a count; JSON Lines reports the record count with the first
//! record's shape; the notebook reports its cell tally with a one-line preview per cell. The full
//! content round-trips. The source map is whole-document for now; a span-preserving parser would map
//! each key, row, or element, which is future work. HTML5 is not well-formed XML and lands in the
//! prose family with a markdown target.

use host_reference_core::{
    content_id, count_tokens, Caps, Edit, Error, Modality, Normalizer, Patch, Semantic, Source,
    SourceMap, Span, SpanSelector, Tier0, Tier1,
};
use serde_json::Value;

pub struct DataNormalizer;

impl DataNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for DataNormalizer {
    fn modality(&self) -> Modality {
        Modality::StructuredData
    }

    fn capabilities(&self) -> Caps {
        // Structured data carries its full semantic shape; the full content round-trips; it is
        // editable; no recognition is involved.
        Caps { round_trip: true, write_back: true, semantic: Semantic::Full, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(
            source.hint,
            Some(
                "json" | "toml" | "yaml" | "yml" | "csv" | "tsv" | "tab" | "xml" | "rss" | "atom"
                    | "ndjson" | "jsonl" | "ipynb"
            )
        )
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let outline = match source.hint {
            Some("csv") => delimited_shape(text, b',')?,
            Some("tsv" | "tab") => delimited_shape(text, b'\t')?,
            Some("yaml" | "yml") => yaml_shape(text)?,
            Some("xml" | "rss" | "atom") => xml_shape(text)?,
            Some("ndjson" | "jsonl") => ndjson_shape(text)?,
            Some("ipynb") => ipynb_shape(text)?,
            Some("toml") => toml_shape(text)?,
            _ => json_shape(text)?,
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
            // Other selectors return the whole document until a span-preserving parser lands.
            _ => (0, text.len()),
        };
        Ok(Tier1 {
            markdown: text[start..end].to_string(),
            source_map: SourceMap { spans: vec![Span { source: id, origin: start..end }] },
        })
    }

    fn put(&self, source: &Source, edit: &Edit) -> Result<Patch, Error> {
        let text = self.text(source)?;
        let start = host_reference_core::floor_boundary(text, edit.at.origin.start);
        let end = host_reference_core::floor_boundary(text, edit.at.origin.end.max(start));
        let mut out = String::with_capacity(text.len());
        out.push_str(&text[..start]);
        out.push_str(&edit.replacement);
        out.push_str(&text[end..]);
        Ok(Patch { bytes: out.into_bytes() })
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn value_shape(v: &Value) -> String {
    let mut out = String::new();
    shape(v, 0, &mut out);
    out
}

fn json_shape(text: &str) -> Result<String, Error> {
    let v: Value = serde_json::from_str(text).map_err(|e| Error::Parse(format!("json: {e}")))?;
    Ok(value_shape(&v))
}

fn yaml_shape(text: &str) -> Result<String, Error> {
    let v: Value = serde_norway::from_str(text).map_err(|e| Error::Parse(format!("yaml: {e}")))?;
    Ok(value_shape(&v))
}

fn toml_shape(text: &str) -> Result<String, Error> {
    let tv: toml::Value = toml::from_str(text).map_err(|e| Error::Parse(format!("toml: {e}")))?;
    let v: Value = serde_json::to_value(tv).map_err(|e| Error::Parse(format!("toml: {e}")))?;
    Ok(value_shape(&v))
}

/// The shape of a value: object keys with their types (sorted), array length with its element
/// type, or a scalar type. Nested objects and object-valued array elements recurse.
fn shape(v: &Value, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                field(k, &map[k], depth, out);
            }
        }
        Value::Array(arr) => {
            let et = arr.first().map(type_name).unwrap_or("empty");
            out.push_str(&format!("{indent}- [array of {} {et}]\n", arr.len()));
            if let Some(first @ Value::Object(_)) = arr.first() {
                shape(first, depth + 1, out);
            }
        }
        scalar => out.push_str(&format!("{indent}- {}\n", type_name(scalar))),
    }
}

fn field(key: &str, val: &Value, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    match val {
        Value::Array(arr) => {
            let et = arr.first().map(type_name).unwrap_or("empty");
            out.push_str(&format!("{indent}- {key}: array[{}] of {et}\n", arr.len()));
            if let Some(first @ Value::Object(_)) = arr.first() {
                shape(first, depth + 1, out);
            }
        }
        Value::Object(_) => {
            out.push_str(&format!("{indent}- {key}: object\n"));
            shape(val, depth + 1, out);
        }
        _ => out.push_str(&format!("{indent}- {key}: {}\n", type_name(val))),
    }
}

fn delimited_shape(text: &str, delim: u8) -> Result<String, Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delim)
        .from_reader(text.as_bytes());
    let headers = rdr
        .headers()
        .map_err(|e| Error::Parse(format!("delimited: {e}")))?
        .clone();
    let mut rows = 0usize;
    for rec in rdr.records() {
        rec.map_err(|e| Error::Parse(format!("delimited: {e}")))?;
        rows += 1;
    }
    let mut out = format!("table: {rows} rows, {} columns\n", headers.len());
    for h in headers.iter() {
        out.push_str(&format!("- {h}\n"));
    }
    Ok(out)
}

/// JSON Lines: each non-empty line is a JSON value. The skeleton is the record count and the shape
/// of the first record, which a homogeneous stream shares.
fn ndjson_shape(text: &str) -> Result<String, Error> {
    let mut count = 0usize;
    let mut first: Option<Value> = None;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value =
            serde_json::from_str(line).map_err(|e| Error::Parse(format!("ndjson: {e}")))?;
        if first.is_none() {
            first = Some(v);
        }
        count += 1;
    }
    let mut out = format!("ndjson: {count} records\n");
    if let Some(v) = first {
        out.push_str("first record:\n");
        out.push_str(&value_shape(&v));
    }
    Ok(out)
}

/// A Jupyter notebook: a JSON document whose `cells` array carries the content. The skeleton is the
/// cell tally and a one-line preview of each cell, which keeps a long notebook token-lean.
fn ipynb_shape(text: &str) -> Result<String, Error> {
    let nb: Value = serde_json::from_str(text).map_err(|e| Error::Parse(format!("ipynb: {e}")))?;
    let cells = nb
        .get("cells")
        .and_then(|c| c.as_array())
        .ok_or_else(|| Error::Parse("ipynb: no cells array".into()))?;
    let (mut code, mut markdown, mut other) = (0usize, 0usize, 0usize);
    let mut lines = String::new();
    for cell in cells {
        let ct = cell.get("cell_type").and_then(|t| t.as_str()).unwrap_or("unknown");
        match ct {
            "code" => code += 1,
            "markdown" => markdown += 1,
            _ => other += 1,
        }
        lines.push_str(&format!("- [{ct}] {}\n", cell_first_line(cell)));
    }
    let mut out = format!("notebook: {} cells ({code} code, {markdown} markdown", cells.len());
    if other > 0 {
        out.push_str(&format!(", {other} other"));
    }
    out.push_str(")\n");
    out.push_str(&lines);
    Ok(out)
}

/// The first non-empty source line of a cell, the `source` being a string or an array of line
/// strings.
fn cell_first_line(cell: &Value) -> String {
    let joined: String = match cell.get("source") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).collect(),
        _ => String::new(),
    };
    joined
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

fn xml_shape(text: &str) -> Result<String, Error> {
    let doc = roxmltree::Document::parse(text).map_err(|e| Error::Parse(format!("xml: {e}")))?;
    let mut out = String::new();
    emit_element(doc.root_element().tag_name().name(), 1, doc.root_element(), 0, &mut out);
    Ok(out)
}

/// One element of the XML tree: its tag (with a count when a sibling tag repeats), then each
/// distinct child tag once in first-seen order, recursing into the first occurrence of each.
fn emit_element(name: &str, count: usize, node: roxmltree::Node, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    if count > 1 {
        out.push_str(&format!("{indent}- {name} (x{count})\n"));
    } else {
        out.push_str(&format!("{indent}- {name}\n"));
    }
    let mut groups: Vec<(&str, usize, roxmltree::Node)> = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        let cn = child.tag_name().name();
        match groups.iter_mut().find(|(n, _, _)| *n == cn) {
            Some(g) => g.1 += 1,
            None => groups.push((cn, 1, child)),
        }
    }
    for (cn, c, first) in groups {
        emit_element(cn, c, first, depth + 1, out);
    }
}
